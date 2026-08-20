use core::fmt::Debug;

use derive_more::Display;

use crate::content::display_width;
use crate::draw::DrawContext;
use crate::scaffold::Scaffold;
use crate::style::Value;
use crate::style::Size;
use crate::style::Gap;
use crate::style::FlexDirection;
use crate::style::FontStyle;
use crate::style::ContentColor;
use crate::event::keyboard::KeyboardEvent;

//--
// Note: `Legend`/`Header`/`Body`/`Footer`/`Content`/`Children` are left out of this prelude —
// they're unimplemented Slot markers (see the TODO below), and `Content` would collide with
// `crate::content::Content`.
pub mod prelude {
    pub use super::Element;
    pub use super::Container;
    pub use super::Text;
    pub use super::Input;
    pub use super::InputIcon;
    pub use super::InputValue;
    pub use super::InputCursor;
    pub use super::Button;
}

pub trait Element {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        |s| s
    }
}

//--
#[derive(Default, Display, Debug)]
pub struct Container;

impl Element for Container {
    //..
}

#[derive(Default, Display, Debug)]
pub struct Text<T>(pub T);

impl<T> Text<T> {
    pub fn new<V: Into<T>>(text: V) -> Self {
        Text(text.into())
    }
    
    pub fn with_content<C: Into<T>>(mut self, value: C) -> Self {
        self.0 = value.into();
        self // etc ..
    }
}

impl<T: AsRef<str>> Text<T> {
    pub fn content(&self) -> &str {
        self.0.as_ref()
    }
}

impl<T: AsRef<str>> Element for Text<T> {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        move |text| {
            text
                // TODO: Allocate content in the DrawContext ..
                .with_content(Some(self.0.as_ref().to_string()))
        }
    }
}

pub struct Input<T>{
    pub value: T,
    pub placeholder: Option<T>,
    /// Whether the blink cursor is in its visible half-cycle this frame — the caller (which
    /// owns the clock everything else's animation runs on, e.g. a spinner) is expected to
    /// recompute this every draw and pass it in, the same way `assistant.rs` already does for
    /// its overlay's pulse. Defaults to `true` (a steady, always-visible cursor) so a caller
    /// that never touches this still gets a sensible result.
    pub cursor_visible: bool,
}

impl<T> Input<T> {
    pub fn new<V: Into<T>>(text: V) -> Self {
        Input {
            value: text.into(),
            placeholder: None,
            cursor_visible: true,
        }
    }

    pub fn with_value<V: Into<T>>(mut self, value: V) -> Self {
        self.value = value.into();
        self // etc ..
    }

    pub fn with_placeholder<P: Into<T>>(mut self, placeholder: P) -> Self {
        self.placeholder = Some(placeholder.into());
        self // etc ..
    }

    pub fn with_cursor_visible(mut self, visible: bool) -> Self {
        self.cursor_visible = visible;
        self // etc ..
    }
}

impl<V: AsRef<str> + Default> Element for Input<V> {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        move |input| {
            let current_value = self.value.as_ref().to_owned();
            let placeholder = self.placeholder.as_ref().map(|placeholder| placeholder.as_ref().to_owned());
            let cursor_glyph = if self.cursor_visible { "_" } else { " " };

            // TODO: Support local allocation for the input's contents ..
            // let some_bump_string = crate::draw::format!(in ctx.arena(), "TODO: {}", "");

            input
                .with_style(FlexDirection::Row)
                .with_style(Gap(Value::from(1)))
                .with_handler::<KeyboardEvent>({
                    move |event| match event.key {
                        _ => {} // tracing::debug!("TODO: Handle keyboard events!"),
                    }
                })
                .with_slot::<InputIcon>(|prefix| {
                    prefix
                        .with_style(FlexDirection::Row)
                        .with_style(Size::width(1))
                        .with_content(Some("$"))
                })
                .with_slot::<InputValue>(move |text| {
                    // The cursor slot is always present (not just once there's typed text) —
                    // it's the "ready to take input" indicator, so an empty input still needs
                    // to show it blinking next to the placeholder.
                    if current_value.is_empty() {
                        text
                            .with_style(FlexDirection::Row)
                            .with_style(FontStyle::Italic)
                            .with_style(ContentColor::from("#555555ff"))
                            .with_content(placeholder)
                    } else {
                        // Display width, not byte length — `current_value` may carry embedded
                        // ANSI styling (e.g. a highlighted `/command` prefix, see `assistant.rs`)
                        // whose escape bytes take up no actual columns.
                        text
                            .with_style(FlexDirection::Row)
                            .with_style(Size::width(display_width(&current_value) + 1))
                            .with_content(Some(current_value))
                    }
                    .with_slot::<InputCursor>(move |cursor| {
                        cursor
                            .with_style(FlexDirection::Row)
                            .with_style(Size::width(1))
                            // .with_style(BackgroundColor::from("#aaaaaaff"))
                            .with_content(Some(cursor_glyph))
                    })
                })
        }
    }
}

/// A pressable control carrying a text label — composed as a plain `Container`-shaped node with
/// its label as `content`, not a generic `Button<T>` the way `Text<T>`/`Input<T>` are generic.
/// Deliberate: a surface doing real native-widget dispatch (an `NSButton`, say) needs to detect
/// "is this node a `Button`" via `Scaffold::get_element::<E: Element + Any>()`, which requires
/// knowing the exact concrete type at the call site — a generic `Button<T>` would make that
/// impossible for a surface that doesn't know what `T` the app composing the tree chose. Carries
/// no handler of its own; the caller registers behavior directly on the composed `Scaffold` via
/// `.with_handler::<escher_core::event::ClickEvent>(..)`, the same way `runtimes/terminal/
/// examples/mouse.rs` already does for its own `ClickEvent`.
#[derive(Default, Debug, Clone)]
pub struct Button {
    pub label: String,
    pub disabled: bool,
}

impl Button {
    pub fn new(label: impl Into<String>) -> Self {
        Button { label: label.into(), disabled: false }
    }

    pub fn with_label(mut self, label: impl Into<String>) -> Self {
        self.label = label.into();
        self // etc..
    }

    /// Data only — a surface reads this via `get_element::<Button>()` to decide native
    /// enabled/disabled state (e.g. `NSButton.isEnabled`). Deliberately doesn't call
    /// `Scaffold::condition(false)`: that controls whether a node renders *at all* (skipped
    /// outright by every surface's slot walk), a different concept from a visible-but-inert
    /// disabled button.
    pub fn with_disabled(mut self, disabled: bool) -> Self {
        self.disabled = disabled;
        self // etc..
    }
}

impl Element for Button {
    fn draw(&self, _: DrawContext) -> impl FnOnce(Scaffold) -> Scaffold {
        move |button| {
            button
                .with_style(FlexDirection::Row)
                .with_content(Some(self.label.clone()))
        }
    }
}

//---
pub struct InputIcon;

pub struct InputValue;

pub struct InputCursor;

//---
// TODO: Move these to escher_scaffold and impl Slot for each.
pub struct Legend;

pub struct Header;

pub struct Body;

pub struct Footer;

pub struct Content;

pub struct Children;
