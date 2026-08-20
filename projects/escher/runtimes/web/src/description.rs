//! A JSON-deserializable description of a `Scaffold` tree. This is the shape `@escher/jsx` (see
//! `packages/jsx`) compiles JSX down to. `parse`/`apply_description` convert it into a real
//! `Scaffold`, used by both `mount_scaffold_from_json` (wasm) and `crate::ssg` (native).

use serde::Deserialize;

use escher_core::scaffold::Scaffold;
use escher_core::style::BackgroundColor;
use escher_core::style::ContentColor;
use escher_core::style::Edge;
use escher_core::style::Flex;
use escher_core::style::FlexDirection;
use escher_core::style::Gap;
use escher_core::style::Margin;
use escher_core::style::Padding;
use escher_core::style::Size;
use escher_core::style::Value;

/// Parses `json` and mounts the resulting scaffold into `root`, replacing its current content.
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen::prelude::wasm_bindgen(js_name = "mountScaffoldFromJson")]
pub fn mount_scaffold_from_json(root: &web_sys::Element, json: &str) -> Result<(), wasm_bindgen::JsValue> {
    let description = parse(json).map_err(|error| wasm_bindgen::JsValue::from_str(&error))?;

    let arena = escher_core::draw::Bump::new();
    let scaffold = apply_description(Scaffold::new_in(&arena), description);

    crate::surface::mount(root, &scaffold)
}

/// Parses a `ScaffoldDescription` from JSON.
pub fn parse(json: &str) -> Result<ScaffoldDescription, String> {
    serde_json::from_str(json).map_err(|error| format!("invalid scaffold JSON: {error}"))
}

/// Deliberately `Deserialize`-only, no `Serialize`: this type exists to cross the wire from a JSX-
/// authoring tool (`@escher/jsx`, or an Ethos script emitting the same shape), not to be
/// constructed directly in application code. Hand-building one in Rust would bypass Escher's own
/// UI composition patterns (`Scaffold::style`/`slot`/`content`, the same ergonomic
/// builder every native surface already composes with) in favor of assembling the wire schema by
/// hand — technically works, but breaks the whole point of having one consistent authoring pattern
/// across surfaces. See `ssg::render_scaffold_to_html` for how to render a real, natively-built
/// `Scaffold` without ever touching this type.
#[derive(Debug, Deserialize)]
pub struct ScaffoldDescription {
    #[serde(default)]
    pub content: Option<String>,
    #[serde(default)]
    pub styles: Vec<StyleDescription>,
    #[serde(default)]
    pub children: Vec<ScaffoldDescription>,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "type", rename_all = "camelCase")]
pub enum StyleDescription {
    Size {
        #[serde(default)]
        width: Option<ValueDescription>,
        #[serde(default)]
        height: Option<ValueDescription>,
    },
    Margin { edge: EdgeDescription, value: ValueDescription },
    Padding { edge: EdgeDescription, value: ValueDescription },
    Gap { value: ValueDescription },
    Flex { grow: f64 },
    FlexDirection { direction: FlexDirectionDescription },
    BackgroundColor { color: String },
    ContentColor { color: String },
}

#[derive(Debug, Deserialize)]
#[serde(tag = "unit", rename_all = "camelCase")]
pub enum ValueDescription {
    Auto,
    Px { value: f64 },
    Percent { value: f64 },
    Fill { value: f64 },
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum EdgeDescription {
    All,
    Top,
    Right,
    Bottom,
    Left,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub enum FlexDirectionDescription {
    Row,
    Column,
}

/// Generic slot marker for JSON-described children. Rendering walks `get_slots()` uniformly, so
/// no per-child type is needed.
struct DescribedSlot;

/// `pub`, not `pub(crate)` — external consumers with their own `Scaffold` arena (e.g. Anvil's
/// terminal renderer) build a real `Scaffold` from a `ScaffoldDescription` this way instead of
/// going through JSON at all when they already have the parsed value in hand.
pub fn apply_description(mut scaffold: Scaffold<'_>, description: ScaffoldDescription) -> Scaffold<'_> {
    for style in description.styles {
        scaffold = apply_style(scaffold, style);
    }

    if let Some(content) = description.content {
        scaffold = scaffold.content(Some(content));
    }

    for child in description.children {
        scaffold = scaffold.slot::<DescribedSlot>(move |slot| apply_description(slot, child));
    }

    scaffold
}

fn apply_style(scaffold: Scaffold<'_>, style: StyleDescription) -> Scaffold<'_> {
    match style {
        StyleDescription::Size { width, height } => scaffold.style(Size(
            width.map(into_value).unwrap_or_default(),
            height.map(into_value).unwrap_or_default(),
            Value::Auto,
        )),
        StyleDescription::Margin { edge, value } => scaffold.style(Margin(into_edge(edge), into_value(value))),
        StyleDescription::Padding { edge, value } => scaffold.style(Padding(into_edge(edge), into_value(value))),
        StyleDescription::Gap { value } => scaffold.style(Gap(into_value(value))),
        StyleDescription::Flex { grow } => scaffold.style(Flex::new(grow)),
        StyleDescription::FlexDirection { direction } => scaffold.style(into_flex_direction(direction)),
        StyleDescription::BackgroundColor { color } => scaffold.style(BackgroundColor::try_from(color.as_str()).unwrap_or_default()),
        StyleDescription::ContentColor { color } => scaffold.style(ContentColor::try_from(color.as_str()).unwrap_or_default()),
    }
}

fn into_value(value: ValueDescription) -> Value {
    match value {
        ValueDescription::Auto => Value::Auto,
        ValueDescription::Px { value } => Value::Px(value.into()),
        ValueDescription::Percent { value } => Value::Percent(value.into()),
        ValueDescription::Fill { value } => Value::Fill(value.into()),
    }
}

fn into_edge(edge: EdgeDescription) -> Edge {
    match edge {
        EdgeDescription::All => Edge::All,
        EdgeDescription::Top => Edge::Top,
        EdgeDescription::Right => Edge::Right,
        EdgeDescription::Bottom => Edge::Bottom,
        EdgeDescription::Left => Edge::Left,
    }
}

fn into_flex_direction(direction: FlexDirectionDescription) -> FlexDirection {
    match direction {
        FlexDirectionDescription::Row => FlexDirection::Row,
        FlexDirectionDescription::Column => FlexDirection::Column,
    }
}
