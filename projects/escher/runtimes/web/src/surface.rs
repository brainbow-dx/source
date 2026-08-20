//! Renders an `escher_core::scaffold::Scaffold` tree into real DOM nodes.
//!
//! Same shape as `escher_bevy::surface::BevySurface`/`escher_terminal::surface::TerminalSurface`:
//! a bump-allocated `Scaffold` is built fresh and walked recursively to emit output, rebuilding
//! the whole subtree on every call rather than diffing. `mount_scaffold` is the wasm-bindgen entry
//! point called by the `<escher-scaffold>` custom element (`scaffold-element.js`) once the wasm
//! module has finished loading.

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;

use web_sys::Document;
use web_sys::Element;
use web_sys::HtmlElement;

use escher_core::draw::Bump;
use escher_core::scaffold::Scaffold;
use escher_core::style::Color as EscherColor;
use escher_core::style::Edge;
use escher_core::style::Flex;
use escher_core::style::FlexDirection;
use escher_core::style::Gap;
use escher_core::style::Margin;
use escher_core::style::Padding;
use escher_core::style::Property;
use escher_core::style::Size;
use escher_core::style::Value;

use crate::default_page::build_page_scaffold;

/// Called by `scaffold-element.js`'s `connectedCallback` with the `<escher-scaffold>` element
/// itself as `root` — clears any previous content and renders the page's placeholder scaffold
/// into it. Superseded by `description::mount_scaffold_from_json` for real pages (any page
/// created with content, e.g. via `@escher/jsx`) — kept as the fallback for pages with no
/// embedded scaffold JSON (`scaffold-element.js` decides which one to call).
#[wasm_bindgen(js_name = "mountScaffold")]
pub fn mount_scaffold(root: &Element) -> Result<(), JsValue> {
    let arena = Bump::new();
    let scaffold = build_page_scaffold(&arena);
    mount(root, &scaffold)
}

/// Shared by `mount_scaffold` and `description::mount_scaffold_from_json`: clears `root` and
/// renders `scaffold` into it fresh.
pub(crate) fn mount(root: &Element, scaffold: &Scaffold) -> Result<(), JsValue> {
    let document = root.owner_document().ok_or_else(|| JsValue::from_str("no owner document"))?;

    while let Some(child) = root.first_child() {
        root.remove_child(&child)?;
    }

    if scaffold.is_enabled() {
        let node = render_node(&document, scaffold)?;
        root.append_child(&node)?;
    }

    Ok(())
}

fn render_node(document: &Document, scaffold: &Scaffold) -> Result<Element, JsValue> {
    let element = document.create_element("div")?;
    apply_styles(&element, scaffold);

    if let Some(content) = scaffold.get_content() {
        element.set_text_content(Some(content.as_str()));
    }

    for (_, child) in scaffold.get_slots().iter() {
        if child.is_enabled() {
            let child_node = render_node(document, child)?;
            element.append_child(&child_node)?;
        }
    }

    Ok(element)
}

fn apply_styles(element: &Element, scaffold: &Scaffold) {
    let Some(html_element) = element.dyn_ref::<HtmlElement>() else { return };
    let style = html_element.style();

    for property in scaffold.get_styles().iter().flat_map(|(_, values)| values) {
        match property {
            Property::Size(size) => apply_size(&style, size),
            Property::Margin(Margin(edge, value)) => apply_edge(&style, "margin", *edge, value),
            Property::Padding(Padding(edge, value)) => apply_edge(&style, "padding", *edge, value),
            Property::Gap(Gap(value)) => {
                let _ = style.set_property("display", "flex");
                let _ = style.set_property("gap", &into_css_length(value));
            }
            Property::Flex(Flex(unit)) => {
                let _ = style.set_property("display", "flex");
                let _ = style.set_property("flex-grow", &unit.0.to_string());
            }
            Property::FlexDirection(direction) => {
                let _ = style.set_property("display", "flex");
                let _ = style.set_property("flex-direction", into_css_flex_direction(*direction));
            }
            Property::BackgroundColor(color) => {
                if let Some(css) = into_css_color(color) {
                    let _ = style.set_property("background-color", &css);
                }
            }
            Property::ContentColor(color) => {
                if let Some(css) = into_css_color(color) {
                    let _ = style.set_property("color", &css);
                }
            }
            // Border/Heading/FontStyle/FontWeight/TextDecorationLine/TextAlign/Overflow/
            // ScrollPosition have no CSS mapping wired up yet — `TerminalSurface`/`BevySurface`
            // are the reference for how each would map if a future page needs them.
            _ => {}
        }
    }
}

fn apply_size(style: &web_sys::CssStyleDeclaration, size: &Size) {
    let Size(width, height, _depth) = size;
    if !matches!(width, Value::Auto) {
        let _ = style.set_property("width", &into_css_length(width));
    }
    if !matches!(height, Value::Auto) {
        let _ = style.set_property("height", &into_css_length(height));
    }
}

fn apply_edge(style: &web_sys::CssStyleDeclaration, property: &str, edge: Edge, value: &Value) {
    let css_value = into_css_length(value);
    let sides: &[&str] = match edge {
        Edge::All => &["top", "right", "bottom", "left"],
        Edge::Top => &["top"],
        Edge::Right => &["right"],
        Edge::Bottom => &["bottom"],
        Edge::Left => &["left"],
        Edge::None => &[],
    };
    for side in sides {
        let _ = style.set_property(&format!("{property}-{side}"), &css_value);
    }
}

fn into_css_length(value: &Value) -> String {
    match value {
        Value::Auto => "auto".to_string(),
        Value::Px(unit) => format!("{}px", unit.0),
        Value::Percent(unit) => format!("{}%", unit.0),
        Value::Fill(_) => "100%".to_string(),
    }
}

fn into_css_flex_direction(direction: FlexDirection) -> &'static str {
    match direction {
        FlexDirection::Row => "row",
        FlexDirection::Column => "column",
    }
}

fn into_css_color(color: &EscherColor) -> Option<String> {
    color.map(|linear| format!("rgba({}, {}, {}, {})", linear.red, linear.green, linear.blue, linear.alpha as f32 / 255.0))
}
