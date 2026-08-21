//! Dev-tool "extension" support: mounts JS/CSS from `.anvil.toml`'s `extensions` directories into
//! every browser tab's webview via `escher_webview::WebView::add_script`. See `spec/.agents/
//! proposals/webview-script-injection-mvp.md` for the design and its deliberate limits — no
//! `chrome.*`/`browser.*` API surface, no manifest-driven per-URL matching. Every script/style
//! runs on every page, the same as a plain userscript.

/// Reads every `.js`/`.css` file directly inside each of `dirs` (non-recursive, no
/// `manifest.json` parsing) and combines them into one script to hand to
/// `WebView::add_script`. CSS becomes a `<style>` element appended via JS at injection time —
/// there's no separate native "inject CSS" API on either webview backend, and layering it on top
/// of the same JS injection mechanism keeps this to one code path instead of two. Files within
/// one directory are read in directory-listing order, which isn't guaranteed consistent across
/// platforms/filesystems — fine for independent scripts, but an extension relying on a specific
/// load order between its own files should combine them into one file itself for now.
pub fn load_extensions(dirs: &[String]) -> String {
    let mut combined = String::new();

    for dir in dirs {
        let entries = match std::fs::read_dir(dir) {
            Ok(entries) => entries,
            Err(error) => {
                tracing::warn!("Could not read extension directory {dir}: {error}");
                continue;
            }
        };

        for entry in entries.flatten() {
            let path = entry.path();
            let Some(extension) = path.extension().and_then(|extension| extension.to_str()) else { continue };

            let contents = match std::fs::read_to_string(&path) {
                Ok(contents) => contents,
                Err(error) => {
                    tracing::warn!("Could not read extension file {}: {error}", path.display());
                    continue;
                }
            };

            match extension {
                "js" => {
                    combined.push_str(&contents);
                    combined.push('\n');
                }
                // JSON-encoded rather than dropped into a raw template string: a stylesheet
                // containing a literal `</script>` or backtick would otherwise break out of it.
                "css" => {
                    let encoded = serde_json::to_string(&contents).unwrap_or_else(|_| "\"\"".to_string());
                    combined.push_str(&format!(
                        "(() => {{ const style = document.createElement('style'); style.textContent = {encoded}; \
                         (document.head ?? document.documentElement).appendChild(style); }})();\n"
                    ));
                }
                _ => {}
            }
        }

        tracing::info!("Loaded extension from {dir}");
    }

    combined
}
