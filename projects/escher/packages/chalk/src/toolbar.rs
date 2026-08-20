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
pub struct PinButton;

/// Builds the toolbar's content onto `root` — `address` seeds the field's current text;
/// `on_toggle_sidebar` fires on the leading collapse/expand button; `on_back`/`on_forward`/
/// `on_refresh` fire on their respective button click, `on_load` fires with the committed text
/// when the address field is submitted (Return). `loading` swaps the refresh glyph to a distinct
/// "in progress" indicator — purely visual feedback (still fires `on_refresh` on click, same as
/// idle), not a real Stop-loading action; the point is that *something* on screen changes the
/// instant a navigation starts, instead of the toolbar looking identical whether a click landed or
/// not. `pinned`/`on_toggle_pinned` are the trailing pin button — whether *this* window should
/// float above every other window; this crate only ever renders the button and reports the click,
/// the actual `WindowLevel` change is entirely the caller's job (it owns the window entity).
pub fn toolbar<'ctx>(
    root: Scaffold<'ctx>,
    address: &str,
    loading: bool,
    pinned: bool,
    on_toggle_sidebar: impl Fn() + 'static,
    on_back: impl Fn() + 'static,
    on_forward: impl Fn() + 'static,
    on_refresh: impl Fn() + 'static,
    on_load: impl Fn(String) + 'static,
    on_toggle_pinned: impl Fn() + 'static,
) -> Scaffold<'ctx> {
    let address = address.to_string();

    root.style(FlexDirection::Row)
        .style(Gap(Value::from(12)))
        // `78`, not the plain `16` a bar with its own titlebar would want: Anvil's browser window
        // (this toolbar's only consumer today) renders under a transparent, fullsize-content-view
        // titlebar (see `spawn_browser_window_on_command`'s own doc comment), so the traffic-light
        // buttons float directly over this bar's own leading edge — 78pt clears their cluster with
        // a little breathing room, the same leading inset Safari/Chrome's own overlaid toolbars use.
        .style(Padding::left(78))
        .style(Padding::right(16))
        // Every child below has no explicit cross-size of its own, so without this they'd stretch
        // to the whole bar's height — comfortable for a button (self-centers its own title
        // regardless), but bad for the address field's text, which read as vertically misaligned
        // once its box got that tall. `7` each side keeps the same proportions this bar always
        // had (`escher_appkit::TOOLBAR_HEIGHT`, 44) while giving every control a compact, shared
        // 30px height instead.
        .style(Padding::top(7))
        .style(Padding::bottom(7))
        .slot::<SidebarToggle>(move |toggle| {
            // `icon`'s symbolic name is a real Lucide icon (see `escher_appkit::icons`'s own doc
            // comment) an icon-aware surface renders instead of `label` — the Unicode glyph stays
            // as `label` regardless, so a surface that doesn't know how to render icons (the
            // terminal, say) still gets a real, working fallback, not a blank button.
            toggle
                .style(Size::width(34))
                .element(Button::new("\u{2630}").with_icon("menu"))
                .handle::<ClickEvent>(move |_| on_toggle_sidebar())
        })
        .slot::<BackButton>(move |back| {
            back.style(Size::width(34))
                .element(Button::new("\u{2039}").with_icon("chevron-left"))
                .handle::<ClickEvent>(move |_| on_back())
        })
        .slot::<ForwardButton>(move |forward| {
            forward
                .style(Size::width(34))
                .element(Button::new("\u{203A}").with_icon("chevron-right"))
                .handle::<ClickEvent>(move |_| on_forward())
        })
        .slot::<RefreshButton>(move |refresh| {
            // `\u{25CC}` (DOTTED CIRCLE) was tried here first and reverted — it's a Unicode
            // *combining-mark placeholder*, meant to carry a diacritic in font-rendering examples,
            // not a standalone glyph; alone it renders as a tiny, oddly-shaped dot in most fonts
            // instead of a clean icon. `\u{25D0}` (CIRCLE WITH LEFT HALF BLACK) is a real Geometric
            // Shapes character that renders consistently as a plain filled half-circle. Only the
            // idle state gets a real icon (`refresh-cw`) — the loading state is a distinct,
            // transient glyph, not worth bundling a second icon asset for.
            let glyph = if loading { "\u{25D0}" } else { "\u{21BB}" };
            let mut button = Button::new(glyph);
            if !loading {
                button = button.with_icon("refresh-cw");
            }
            refresh.style(Size::width(34)).element(button).handle::<ClickEvent>(move |_| on_refresh())
        })
        .slot::<AddressField>(move |field| {
            field
                .style(escher_core::style::Flex::new(1))
                .element(Input::<String>::new(address.clone()).with_placeholder("Search or enter address".to_string()))
                .handle::<SubmitEvent>(move |SubmitEvent(text)| on_load(text.clone()))
        })
        .slot::<PinButton>(move |pin| {
            // No separate "pinned" icon/glyph — `Button::active` (see its own doc comment) tints
            // the same `pin` icon persistently instead, so this reads as toggled on without
            // needing a second bundled icon asset just for the "on" state.
            //
            // `label` is plain text, not the 📌 emoji it used to be — confirmed live as a real
            // problem: on an icon-aware surface this text is never actually shown (the icon
            // replaces it outright, see `escher_appkit::icons`'s own doc comment), but *when*
            // it's shown at all — an icon-unaware surface, or the icon-aware one falling back
            // for a reason not yet fully root-caused — a color emoji stood out as the one loud,
            // saturated thing in an otherwise monochrome toolbar, exactly backwards from a rare,
            // secondary toggle. Plain text degrades to *quiet*, matching every other toolbar
            // glyph's fallback behavior (‹, ›, ↻, ☰ are already plain characters with no color of
            // their own).
            pin.style(Size::width(34))
                .element(Button::new("Pin").with_icon("pin").with_active(pinned))
                .handle::<ClickEvent>(move |_| on_toggle_pinned())
        })
}
