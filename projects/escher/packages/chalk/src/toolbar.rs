//! Composes a browser toolbar (sidebar toggle, back/forward/refresh + address field) as a real
//! `escher_core::Scaffold` tree — built entirely from `escher_core::element::{Button, Input}`, so
//! any surface that already knows how to render those (every one of them does) renders this
//! correctly with no surface-specific code at all. First component moved into `escher-chalk`; see
//! this crate's own doc comment for why it qualifies and what doesn't yet.
//!
//! Named `toolbar`, not `chrome` — "chrome" (browser UI *around* the page) collides badly with
//! Chrome the browser once a project is already talking about actual browser features; `toolbar`
//! says the same thing unambiguously.

use escher_core::element::{Button, Input};
use escher_core::event::{ClickEvent, SubmitEvent};
use escher_core::scaffold::Scaffold;
use escher_core::style::{FlexDirection, Gap, Padding, Size, Value};

pub struct SidebarToggle;
pub struct BackButton;
pub struct ForwardButton;
pub struct RefreshButton;
pub struct AddressField;

/// Builds the toolbar's content onto `root` — `address` seeds the field's current text;
/// `on_toggle_sidebar` fires on the leading collapse/expand button; `on_back`/`on_forward`/
/// `on_refresh` fire on their respective button click, `on_load` fires with the committed text
/// when the address field is submitted (Return). `loading` swaps the refresh glyph to a distinct
/// "in progress" indicator — purely visual feedback (still fires `on_refresh` on click, same as
/// idle), not a real Stop-loading action; the point is that *something* on screen changes the
/// instant a navigation starts, instead of the toolbar looking identical whether a click landed or
/// not.
pub fn toolbar<'ctx>(
    root: Scaffold<'ctx>,
    address: &str,
    loading: bool,
    on_toggle_sidebar: impl Fn() + 'static,
    on_back: impl Fn() + 'static,
    on_forward: impl Fn() + 'static,
    on_refresh: impl Fn() + 'static,
    on_load: impl Fn(String) + 'static,
) -> Scaffold<'ctx> {
    let address = address.to_string();

    root.with_style(FlexDirection::Row)
        .with_style(Gap(Value::from(10)))
        .with_style(Padding::left(14))
        .with_style(Padding::right(14))
        .with_slot::<SidebarToggle>(move |toggle| {
            toggle
                .with_style(Size::width(34))
                .with_element(Button::new("\u{2630}"))
                .with_handler::<ClickEvent>(move |_| on_toggle_sidebar())
        })
        .with_slot::<BackButton>(move |back| {
            back.with_style(Size::width(34))
                .with_element(Button::new("\u{2039}"))
                .with_handler::<ClickEvent>(move |_| on_back())
        })
        .with_slot::<ForwardButton>(move |forward| {
            forward
                .with_style(Size::width(34))
                .with_element(Button::new("\u{203A}"))
                .with_handler::<ClickEvent>(move |_| on_forward())
        })
        .with_slot::<RefreshButton>(move |refresh| {
            // `\u{25CC}` (DOTTED CIRCLE) was tried here first and reverted — it's a Unicode
            // *combining-mark placeholder*, meant to carry a diacritic in font-rendering examples,
            // not a standalone glyph; alone it renders as a tiny, oddly-shaped dot in most fonts
            // instead of a clean icon. `\u{25D0}` (CIRCLE WITH LEFT HALF BLACK) is a real Geometric
            // Shapes character that renders consistently as a plain filled half-circle.
            let glyph = if loading { "\u{25D0}" } else { "\u{21BB}" };
            refresh.with_style(Size::width(34)).with_element(Button::new(glyph)).with_handler::<ClickEvent>(move |_| on_refresh())
        })
        .with_slot::<AddressField>(move |field| {
            field
                .with_style(escher_core::style::Flex::new(1))
                .with_element(Input::<String>::new(address.clone()).with_placeholder("Search or enter address".to_string()))
                .with_handler::<SubmitEvent>(move |SubmitEvent(text)| on_load(text.clone()))
        })
}
