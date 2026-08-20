//! The `/shape` command: runs Ethos's UXML/USS codegen tool and fans the one JSON result out to
//! all three renderers. See `AppState::spawn_shape_command` in `main.rs` for the orchestration
//! (chat messages, `pending_scenes`, `Page::Process` switch) — this module is just the actual
//! work: run the script, parse its output, produce the terminal block / web page / Unity assets.
//! See `projects/ethos/spec/agents/proposals/uxml-uss-codegen.md` for why the codegen logic
//! itself lives in Ethos, not here.

use std::path::Path;

use color_eyre::owo_colors::OwoColorize;

use crate::process;
use crate::LineBuffer;

const JSX_SCRIPT: &str = "commands/shape.tsx";
const ETHOS_TRANSFORM_SCRIPT: &str = "tools/codegen/uxml/from-description.ts";
const SHAPE_WEB_PORT: u16 = 4001;

/// Both `shape.tsx` and `from-description.ts` are free to `console.log` as much progress as they
/// want — it already streams live into `process_buffer` (`run_deno_command`'s real `deno run`
/// child streams every stdout line as it arrives, same as stderr; `run_js_command`'s embedded
/// engine pipes `console.log` into the same buffer separately from its return value — see that
/// function's own doc comment), so there's no reason to make script authors think about stdout/
/// stderr separation. The one real payload each script produces is its `run()` export's return
/// value — for `run_deno_command`'s real `deno run` child that's still the last line of a
/// captured, possibly-`console.log`-noisy stdout, so `last_line` still matters there; for
/// `run_js_command`'s embedded engine the return value never has any `console.log` output mixed
/// in to begin with, so `last_line` is a harmless no-op on it, kept only so both call sites below
/// can go through the same helper.
fn last_line(output: &str) -> &str {
    output.lines().next_back().unwrap_or(output)
}

/// Runs on a background thread (via `tokio::task::spawn_blocking`, same as `run_js_command`) —
/// see `AppState::spawn_shape_command`. Returns the URL of the written web page on success, for
/// the caller to push onto `pending_scenes`.
///
/// Two-step pipeline — "build the Scaffold" and "compile it to a target format" are different
/// jobs, done in different projects:
/// 1. `commands/shape.tsx` (real `deno run`, JSX-authored, lives in *this* project since this is
///    the command that uses it) builds the actual `ScaffoldDescription` JSON.
/// 2. That JSON gets handed, unmodified, to `ethos/tools/codegen/uxml/from-description.ts` (via
///    `run_js_command`'s embedded engine) — a pure transform with no authored content of its
///    own — which returns `{uxml, uss}`.
///
/// Previously one Ethos script did both (authored the demo shape *and* compiled it), which
/// worked but put content-authoring in the wrong project. See
/// `projects/ethos/spec/agents/proposals/uxml-uss-codegen.md`.
pub(crate) fn run_shape_command(process_buffer: &LineBuffer) -> Result<String, String> {
    let deno_output = process::run_deno_command(Path::new(JSX_SCRIPT), "", "shape (jsx → scaffold)", process_buffer)?;
    let description: serde_json::Value =
        serde_json::from_str(last_line(&deno_output)).map_err(|error| format!("shape.tsx returned invalid JSON: {error}"))?;

    let transform_output =
        process::run_js_command(Path::new(ETHOS_TRANSFORM_SCRIPT), last_line(&deno_output), "shape (scaffold → uxml)", process_buffer)?;
    let transform: serde_json::Value =
        serde_json::from_str(last_line(&transform_output)).map_err(|error| format!("from-description.ts returned invalid JSON: {error}"))?;

    let uxml = transform.get("uxml").and_then(serde_json::Value::as_str).ok_or("missing `uxml` field in transform output")?;
    let uss = transform.get("uss").and_then(serde_json::Value::as_str).ok_or("missing `uss` field in transform output")?;

    render_shape_block(&description, process_buffer);

    process_buffer.push_line("Writing Unity assets...".to_string());
    write_unity_shape_assets(uxml, uss)?;

    process_buffer.push_line("Writing web page...".to_string());
    let url = write_web_shape_page(&description)?;

    process_buffer.push_line("Done.".to_string());
    Ok(url)
}

/// Renders the shape as a plain colored block of terminal rows, followed by any caption text
/// found in the tree — read directly off the same `description` JSON the web/Unity legs consume,
/// not routed through `escher-terminal`'s real `Scaffold`/`Property` rendering pipeline (that
/// would need a deeper integration into how this app's Body area is composed than tonight's scope
/// covers), so this is a deliberately simpler, honest approximation: same shared source of truth,
/// hand-rolled ANSI instead of a real `TerminalSurface` draw. Worth revisiting — see
/// `escher/spec/ROADMAP.md`.
///
/// Walks the whole tree rather than reading `description.styles` directly — `shape-demo.ts`
/// nests the actual colored box one level down (`children: [BOX, CAPTION]`, so the shape
/// explains itself instead of being an unlabeled swatch), so a shallow top-level read
/// would silently miss it. `find_node_with_background_color` finds the box specifically (the one
/// node with a `backgroundColor` style) rather than separately hunting for "a color" and "a size"
/// across the whole tree, which would risk pairing the box's color with the caption's own
/// unrelated `size` style if traversal order ever changed.
fn render_shape_block(description: &serde_json::Value, process_buffer: &LineBuffer) {
    let box_node = find_node_with_background_color(description).unwrap_or(description);
    let styles = box_node.get("styles").and_then(serde_json::Value::as_array).cloned().unwrap_or_default();

    let mut color = (122u8, 162u8, 247u8);
    let mut width_px = 240.0_f64;
    let mut height_px = 140.0_f64;

    for style in &styles {
        match style.get("type").and_then(serde_json::Value::as_str) {
            Some("backgroundColor") => {
                if let Some(parsed) = style.get("color").and_then(serde_json::Value::as_str).and_then(parse_hex_color) {
                    color = parsed;
                }
            }
            Some("size") => {
                if let Some(value) = style.get("width").and_then(|w| w.get("value")).and_then(serde_json::Value::as_f64) {
                    width_px = value;
                }
                if let Some(value) = style.get("height").and_then(|h| h.get("value")).and_then(serde_json::Value::as_f64) {
                    height_px = value;
                }
            }
            _ => {}
        }
    }

    // Rough px→terminal-cell conversion (~8px/16px per cell, a common monospace ratio) — a
    // terminal has no real notion of "pixels," so this is deliberately approximate, just enough
    // to render the same shape's proportions as a visible block.
    let cols = ((width_px / 8.0).round() as usize).max(1);
    let rows = ((height_px / 16.0).round() as usize).max(1);

    for _ in 0..rows {
        process_buffer.push_line(format!("{}", " ".repeat(cols).on_truecolor(color.0, color.1, color.2)));
    }

    for caption in collect_content_strings(description) {
        process_buffer.push_line(String::new());
        for line in escher_terminal::text_wrap::wrap_words(&caption, 78) {
            process_buffer.push_line(line);
        }
    }
}

/// Depth-first search for the first node whose own `styles` include a `backgroundColor` — that's
/// uniquely the box, regardless of how deep it's nested or how many siblings (a caption, future
/// additions) sit alongside it.
fn find_node_with_background_color(node: &serde_json::Value) -> Option<&serde_json::Value> {
    let has_background_color = node
        .get("styles")
        .and_then(serde_json::Value::as_array)
        .is_some_and(|styles| styles.iter().any(|style| style.get("type").and_then(serde_json::Value::as_str) == Some("backgroundColor")));

    if has_background_color {
        return Some(node);
    }

    node.get("children")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find_map(find_node_with_background_color)
}

/// Every `content` string anywhere in the tree, depth-first — `shape-demo.ts` only ever puts one
/// on the caption node today, but this doesn't assume that stays true.
fn collect_content_strings(node: &serde_json::Value) -> Vec<String> {
    let mut out = Vec::new();

    if let Some(content) = node.get("content").and_then(serde_json::Value::as_str) {
        out.push(content.to_string());
    }

    for child in node.get("children").and_then(serde_json::Value::as_array).into_iter().flatten() {
        out.extend(collect_content_strings(child));
    }

    out
}

fn parse_hex_color(hex: &str) -> Option<(u8, u8, u8)> {
    let hex = hex.trim_start_matches('#');
    if hex.len() != 6 {
        return None;
    }
    let r = u8::from_str_radix(&hex[0..2], 16).ok()?;
    let g = u8::from_str_radix(&hex[2..4], 16).ok()?;
    let b = u8::from_str_radix(&hex[4..6], 16).ok()?;
    Some((r, g, b))
}

#[cfg(test)]
mod tests {
    use super::collect_content_strings;
    use super::find_node_with_background_color;
    use super::last_line;
    use super::parse_hex_color;

    #[test]
    fn last_line_skips_progress_output() {
        // The exact shape stdout takes once `console.log` progress lines are involved —
        // `ethos-cli run-command` prints a script's `run()` return value via `print!` (no added
        // newline), strictly after any `console.log` calls it made.
        let stdout = "Parsing scaffold description...\nCompiling to UXML/USS...\nDone.\n{\"uxml\":\"a\",\"uss\":\"b\"}";
        assert_eq!(last_line(stdout), r#"{"uxml":"a","uss":"b"}"#);
    }

    #[test]
    fn last_line_handles_a_single_line() {
        assert_eq!(last_line("{}"), "{}");
    }

    /// The real, current `description` field `ethos/tools/codegen/uxml/shape-demo.ts` emits,
    /// captured from a real `ethos-cli run-command` run (the box has a sibling caption label,
    /// both nested one level under `children`) — kept as one shared
    /// literal so every test below exercises the actual shape this app will really see, not a
    /// hand-simplified stand-in that could drift from it unnoticed.
    fn real_shape_demo_json() -> serde_json::Value {
        serde_json::from_str(include_str!("../tests/fixtures/shape_demo_description.json")).expect("fixture must be valid JSON")
    }

    #[test]
    fn parses_the_actual_shape_demo_color() {
        // The exact literal `ethos/tools/codegen/uxml/shape-demo.ts` emits — if that ever drifts
        // out of sync with what this parser accepts, this is the test that should catch it.
        assert_eq!(parse_hex_color("#7aa2f7"), Some((0x7a, 0xa2, 0xf7)));
    }

    #[test]
    fn accepts_missing_leading_hash() {
        assert_eq!(parse_hex_color("7aa2f7"), Some((0x7a, 0xa2, 0xf7)));
    }

    #[test]
    fn rejects_wrong_length() {
        assert_eq!(parse_hex_color("#fff"), None);
        assert_eq!(parse_hex_color("#7aa2f71"), None);
    }

    #[test]
    fn rejects_non_hex_digits() {
        assert_eq!(parse_hex_color("#zzzzzz"), None);
    }

    #[test]
    fn shape_demo_description_renders_via_escher_web() {
        // What `write_web_shape_page` actually feeds `escher_web::ssg::render_page_to_html`.
        // Lives here rather than in `escher-web` itself — an equivalent test added directly to
        // `runtimes/web/src/ssg.rs` broke that crate's own `cbindgen`-based build script (a real,
        // reproducible bug, logged in `escher/spec/ROADMAP.md`); `apps/anvil` has no build script
        // of its own, so testing the same call from here sidesteps it entirely.
        let json = serde_json::to_string(&real_shape_demo_json()).expect("fixture must re-serialize");

        let html = escher_web::ssg::render_page_to_html(&json).expect("shape description must render");

        assert!(html.contains("width: 240px"), "missing box width in:\n{html}");
        assert!(html.contains("height: 140px"), "missing box height in:\n{html}");
        assert!(html.contains("background-color: rgba("), "missing background-color in:\n{html}");
        assert!(html.contains("Escher demo shape"), "missing caption text in:\n{html}");
    }

    #[test]
    fn finds_the_box_nested_under_children() {
        // The regression this test exists to catch: `render_shape_block` used to read
        // `description.styles` directly, which broke the moment the box moved one level down
        // under `children` when the caption was added — it would've silently kept
        // rendering stale hardcoded defaults instead of the real, current shape.
        let description = real_shape_demo_json();
        let box_node = find_node_with_background_color(&description).expect("box must be found");

        let styles = box_node.get("styles").and_then(serde_json::Value::as_array).expect("box must have styles");
        let has_background_color = styles.iter().any(|style| style.get("type").and_then(serde_json::Value::as_str) == Some("backgroundColor"));
        assert!(has_background_color, "found node isn't actually the box: {box_node}");

        let width = box_node.get("styles").and_then(|s| s.as_array()).and_then(|styles| {
            styles
                .iter()
                .find(|style| style.get("type").and_then(serde_json::Value::as_str) == Some("size"))
                .and_then(|style| style.get("width"))
                .and_then(|w| w.get("value"))
                .and_then(serde_json::Value::as_f64)
        });
        assert_eq!(width, Some(240.0), "picked up the caption's size (420) instead of the box's (240)");
    }

    #[test]
    fn collects_the_caption_text() {
        let description = real_shape_demo_json();
        let captions = collect_content_strings(&description);
        assert_eq!(captions.len(), 1, "expected exactly one caption, got {captions:?}");
        assert!(captions[0].starts_with("Escher demo shape:"));
    }

    #[test]
    fn wrap_words_never_exceeds_width() {
        let description = real_shape_demo_json();
        let caption = &collect_content_strings(&description)[0];

        let lines = escher_terminal::text_wrap::wrap_words(caption, 78);

        assert!(lines.len() > 1, "a ~400-char caption should wrap into more than one line");
        for line in &lines {
            assert!(line.chars().count() <= 78, "line exceeds width: {line:?}");
        }
        assert_eq!(lines.join(" "), *caption, "wrapping must not drop or reorder words");
    }
}

/// Writes only new asset files under `Assets/UI/Generated/` — no `.cs` change, no `.unity` scene
/// edit — since Aby's Unity Editor may be open interactively when this runs (new non-script
/// assets don't trigger a domain reload the way a `.cs` change would). Mirrors
/// `escher-unity/src/bin/export_shape.rs`'s own file-write logic exactly (duplicated rather than
/// shared — that binary already re-runs the Ethos script itself, so sharing would mean an extra
/// crate dependency for two `fs::write` calls).
fn write_unity_shape_assets(uxml: &str, uss: &str) -> Result<(), String> {
    let unity_project = Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../aby/runtimes/unity");
    let output_dir = unity_project.join("Assets/UI/Generated");

    std::fs::create_dir_all(&output_dir).map_err(|error| format!("failed to create Assets/UI/Generated: {error}"))?;
    std::fs::write(output_dir.join("Shape.uxml"), uxml).map_err(|error| format!("failed to write Shape.uxml: {error}"))?;
    std::fs::write(output_dir.join("Shape.uss"), uss).map_err(|error| format!("failed to write Shape.uss: {error}"))?;

    Ok(())
}

/// Writes a static HTML page (via `escher_web::ssg::render_page_to_html`, the same renderer
/// `escher-web`'s own SSG path uses) into the directory served by a locally-running `escher-web`
/// `serve` example on `SHAPE_WEB_PORT` — a manual one-time prerequisite (see the proposal doc /
/// `ROADMAP.md`), not something this app spawns itself, to keep this change scoped to "write a
/// file," not "own a background server's lifecycle."
fn write_web_shape_page(description: &serde_json::Value) -> Result<String, String> {
    let json = serde_json::to_string(description).map_err(|error| format!("failed to serialize shape description: {error}"))?;
    let html = escher_web::ssg::render_page_to_html(&json)?;

    let web_root = Path::new(env!("CARGO_MANIFEST_DIR")).join(".output/shape-demo");
    std::fs::create_dir_all(&web_root).map_err(|error| format!("failed to create shape-demo web root: {error}"))?;
    std::fs::write(web_root.join("shape.html"), html).map_err(|error| format!("failed to write shape.html: {error}"))?;

    Ok(format!("http://127.0.0.1:{SHAPE_WEB_PORT}/shape.html"))
}
