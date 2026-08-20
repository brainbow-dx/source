//! Renders a `description::ScaffoldDescription` to a static HTML string. No browser or wasm
//! required. Mirrors the style to CSS mapping in `surface.rs`'s DOM-mounting path.

use std::fmt::Write;

use escher_core::draw::Bump;
use escher_core::scaffold::Scaffold;
use escher_core::style::Color as EscherColor;
use escher_core::style::Edge;
use escher_core::style::FlexDirection;
use escher_core::style::Property;
use escher_core::style::Size;
use escher_core::style::Value;

use crate::description;

/// Renders `json` to a complete HTML document. Use [`render_fragment`] to embed the markup
/// inside an existing page instead. For content authored directly in Rust (not crossing the wire
/// as JSX-compiled JSON), build a real `Scaffold` and use [`render_scaffold_to_html`] instead of
/// hand-assembling a `ScaffoldDescription` to feed this — see that type's own doc comment for why.
pub fn render_page_to_html(json: &str) -> Result<String, String> {
    let fragment = render_fragment(json)?;
    Ok(wrap_document(&fragment))
}

/// Renders `json` to just the scaffold's own markup, with no surrounding document. For
/// embedding inside another page's `<body>`.
pub fn render_fragment(json: &str) -> Result<String, String> {
    let description = description::parse(json)?;
    let arena = Bump::new();
    let scaffold = description::apply_description(Scaffold::new_in(&arena), description);

    Ok(render_scaffold_fragment(&scaffold))
}

/// Renders an already-built `Scaffold` (composed the normal way, `style`/`slot`/$
/// `content`) to a complete HTML document — the Rust-side equivalent of [`render_page_to_html`]
/// for content that was never JSON to begin with. Exists specifically so a native Rust caller
/// never has to construct a `ScaffoldDescription` by hand just to reach this renderer.
pub fn render_scaffold_to_html(scaffold: &Scaffold) -> String {
    wrap_document(&render_scaffold_fragment(scaffold))
}

/// Renders an already-built `Scaffold` to just its own markup, no surrounding document — the
/// `Scaffold`-accepting equivalent of [`render_fragment`].
pub fn render_scaffold_fragment(scaffold: &Scaffold) -> String {
    let mut html = String::new();
    if scaffold.is_enabled() {
        render_node(&mut html, scaffold);
    }
    html
}

fn wrap_document(fragment: &str) -> String {
    format!(
        "<!doctype html>\n\
         <html lang=\"en\">\n\
         <head>\n\
         <meta charset=\"utf-8\" />\n\
         <title>Escher page</title>\n\
         </head>\n\
         <body style=\"margin: 0; background: #000;\">\n\
         {fragment}\n\
         </body>\n\
         </html>\n"
    )
}

/// Renders the placeholder scaffold from `default_page`, for pages with no custom content.
pub fn render_default_fragment() -> String {
    let arena = Bump::new();
    let scaffold = crate::default_page::build_page_scaffold(&arena);

    let mut html = String::new();
    if scaffold.is_enabled() {
        render_node(&mut html, &scaffold);
    }
    html
}

/// Real, semantic tags where a scaffold node carries an element that maps to one — see
/// `surface.rs`'s own `render_node` (the live-DOM equivalent) for the full reasoning: a plain
/// `<div>` is HTML's own correct element for "no inherent semantics," not a fallback, and no
/// custom element is ever created here — this crate has no content that needs one.
fn render_node(out: &mut String, scaffold: &Scaffold) {
    if let Some(button) = scaffold.get_element::<escher_core::element::Button>() {
        let text = scaffold.get_content().map(|content| content.as_str()).unwrap_or(button.label.as_str());
        let disabled = if button.disabled { " disabled" } else { "" };
        let _ = write!(out, "<button style=\"{}\"{disabled}>{}</button>", style_attribute(scaffold), escape_html(text));
        return;
    }

    if let Some(input) = scaffold.get_element::<escher_core::element::Input<String>>() {
        let placeholder = input.placeholder.as_deref().map(|p| format!(" placeholder=\"{}\"", escape_html(p))).unwrap_or_default();
        let _ = write!(out, "<input style=\"{}\" value=\"{}\"{placeholder} />", style_attribute(scaffold), escape_html(&input.value));
        return; // void element — no children
    }

    let _ = write!(out, "<div style=\"{}\">", style_attribute(scaffold));

    if let Some(content) = scaffold.get_content() {
        let _ = write!(out, "{}", escape_html(content.as_str()));
    }

    for (_, child) in scaffold.get_slots().iter() {
        if child.is_enabled() {
            render_node(out, child);
        }
    }

    let _ = write!(out, "</div>");
}

/// Maps the same `Property` subset as `surface.rs`'s `apply_styles`.
fn style_attribute(scaffold: &Scaffold) -> String {
    let mut declarations = Vec::new();

    for property in scaffold.get_styles().iter().flat_map(|(_, values)| values) {
        match property {
            Property::Size(size) => {
                let Size(width, height, _depth) = size;
                if !matches!(width, Value::Auto) {
                    declarations.push(format!("width: {}", css_length(width)));
                }
                if !matches!(height, Value::Auto) {
                    declarations.push(format!("height: {}", css_length(height)));
                }
            }
            Property::Margin(margin) => push_edge(&mut declarations, "margin", margin.0, &margin.1),
            Property::Padding(padding) => push_edge(&mut declarations, "padding", padding.0, &padding.1),
            Property::Gap(gap) => {
                declarations.push("display: flex".to_string());
                declarations.push(format!("gap: {}", css_length(&gap.0)));
            }
            Property::Flex(flex) => {
                declarations.push("display: flex".to_string());
                declarations.push(format!("flex-grow: {}", flex.0.0));
            }
            Property::FlexDirection(direction) => {
                declarations.push("display: flex".to_string());
                declarations.push(format!("flex-direction: {}", css_flex_direction(*direction)));
            }
            Property::BackgroundColor(color) => {
                if let Some(css) = css_color(color) {
                    declarations.push(format!("background-color: {css}"));
                }
            }
            Property::ContentColor(color) => {
                if let Some(css) = css_color(color) {
                    declarations.push(format!("color: {css}"));
                }
            }
            _ => {}
        }
    }

    declarations.join("; ")
}

fn push_edge(declarations: &mut Vec<String>, property: &str, edge: Edge, value: &Value) {
    let css_value = css_length(value);
    let sides: &[&str] = match edge {
        Edge::All => &["top", "right", "bottom", "left"],
        Edge::Top => &["top"],
        Edge::Right => &["right"],
        Edge::Bottom => &["bottom"],
        Edge::Left => &["left"],
        Edge::None => &[],
    };
    for side in sides {
        declarations.push(format!("{property}-{side}: {css_value}"));
    }
}

fn css_length(value: &Value) -> String {
    match value {
        Value::Auto => "auto".to_string(),
        Value::Px(unit) => format!("{}px", unit.0),
        Value::Percent(unit) => format!("{}%", unit.0),
        Value::Fill(_) => "100%".to_string(),
    }
}

fn css_flex_direction(direction: FlexDirection) -> &'static str {
    match direction {
        FlexDirection::Row => "row",
        FlexDirection::Column => "column",
    }
}

fn css_color(color: &EscherColor) -> Option<String> {
    color.map(|linear| format!("rgba({}, {}, {}, {})", linear.red, linear.green, linear.blue, linear.alpha as f32 / 255.0))
}

/// Escapes the characters that would break out of a text node or a double-quoted attribute value.
/// Fixed to actually cover the attribute case (`"` → `&quot;`) now that `render_node` uses this for
/// `<input value="...">`/`placeholder="..."` too, not just text nodes — this doc comment already
/// claimed attribute-safety before that was true; now it is.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;").replace('"', "&quot;")
}

// No `#[cfg(test)] mod tests` here — adding one to cover the shape-demo description round-trip
// breaks this crate's own build script. `scripts/build.rs`'s
// `cbindgen::Builder::with_crate(CARGO_MANIFEST_DIR).generate()` genuinely errors (a real
// `cbindgen::bindgen::error::Error`, not a flake) specifically when a `#[cfg(test)] mod tests {
// use super::...; }` block exists in this file; `eyre`'s own "no hook installed" panic on top of
// that just obscures the real error. Doesn't affect this crate as an ordinary lib dependency
// (`cargo build -p escher-anvil` links it fine) — only `cargo test -p escher-web`/`cargo run
// --example` on this crate directly. Logged in `escher/spec/ROADMAP.md`; the actual round-trip
// this would have tested was verified instead via `escher-unity/src/bin/export_shape.rs` (a real
// `ethos-cli run-command` → JSON → file-write path) and by tracing `description.rs`'s
// `StyleDescription`/`apply_style` match arms against this file's own `style_attribute` match
// arms by hand.
