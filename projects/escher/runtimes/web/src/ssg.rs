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
/// inside an existing page instead.
pub fn render_page_to_html(json: &str) -> Result<String, String> {
    let fragment = render_fragment(json)?;

    Ok(format!(
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
    ))
}

/// Renders `json` to just the scaffold's own markup, with no surrounding document. For
/// embedding inside another page's `<body>`.
pub fn render_fragment(json: &str) -> Result<String, String> {
    let description = description::parse(json)?;
    let arena = Bump::new();
    let scaffold = description::apply_description(Scaffold::new_in(&arena), description);

    let mut html = String::new();
    if scaffold.is_enabled() {
        render_node(&mut html, &scaffold);
    }
    Ok(html)
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

fn render_node(out: &mut String, scaffold: &Scaffold) {
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

/// Escapes only the characters that would break out of a text node or attribute value.
fn escape_html(text: &str) -> String {
    text.replace('&', "&amp;").replace('<', "&lt;").replace('>', "&gt;")
}
