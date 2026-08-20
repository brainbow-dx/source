//! Composes a vertical tab strip (favicon + title + close button per row) as a real
//! `escher_core::Scaffold` tree — one row per open scene, backed by `crate::surface::TabRowMarker`
//! for click-to-select/drag-to-reorder (see `crate::views::TabRowView`'s own doc comment for the
//! interaction model) and `crate::surface::FaviconImage` for the cached-favicon display. The active
//! tab gets a highlighted background — `TabRowMarker::selected`, painted by `TabRowView::drawRect:`
//! directly (no `escher_core::style::BackgroundColor`/`CALayer` involved; a plain `NSBezierPath`
//! fill is enough for a flat highlight).
//!
//! Deliberately *not* in `escher-chalk` alongside `toolbar` — its favicon and drag-to-reorder
//! interactions are only meaningful through `crate::surface`'s AppKit-specific marker elements,
//! which no other surface (Bevy, Terminal, web) knows how to interpret. Moving this file unchanged
//! into a "shared, portable component" crate would just mislabel that coupling, not remove it. It
//! belongs there once `escher-core` grows a real generic image element and a generic drag gesture
//! this composition can build on instead.

use escher_chalk::tabs::TabInfo;
use escher_core::element::Button;
use escher_core::event::ClickEvent;
use escher_core::scaffold::Scaffold;
use escher_core::style::{FlexDirection, Flex, Gap, Padding, Size, Value};

use crate::surface::{FaviconImage, TabRowMarker, TabRowReleased};

pub const ROW_HEIGHT: f64 = 38.0;
const ROW_GAP: f64 = 3.0;

pub struct TabRow;
pub struct TabFavicon;
pub struct TabTitle;
pub struct TabClose;
pub struct NewTabButton;

/// Builds the tab strip's content onto `root`. `active` is the currently-selected tab's `id` (if
/// any), used only to highlight that row — see `crate::surface::TabRowMarker::selected`.
/// `on_select`/`on_close` fire with the clicked tab's `id`; `on_reorder` fires with
/// `(id, positions)` — `positions` is how many rows up (negative) or down (positive) the drag
/// ended relative to where it started, already converted from raw pixel displacement using this
/// module's own row geometry, so the caller just needs to move that tab `positions` slots in its
/// own list (clamped to bounds) rather than re-deriving it from pixels itself. `on_new_tab` fires
/// when the trailing "+" row is clicked.
pub fn tab_strip<'ctx>(
    root: Scaffold<'ctx>,
    tabs: &[TabInfo],
    active: Option<u64>,
    on_select: impl Fn(u64) + Clone + 'static,
    on_close: impl Fn(u64) + Clone + 'static,
    on_reorder: impl Fn(u64, i32) + Clone + 'static,
    on_new_tab: impl Fn() + 'static,
) -> Scaffold<'ctx> {
    let mut root = root.with_style(FlexDirection::Column).with_style(Gap(Value::from(ROW_GAP))).with_style(Padding::all(6));

    for tab in tabs {
        let tab_id = tab.id;
        let title = tab.title.clone();
        let host = tab.host.clone();
        let on_select = on_select.clone();
        let on_close = on_close.clone();
        let on_reorder = on_reorder.clone();

        root = root.with_slot::<TabRow>(move |row| {
            row.with_element(TabRowMarker { selected: Some(tab_id) == active })
                .with_style(FlexDirection::Row)
                .with_style(Size::height(ROW_HEIGHT))
                .with_style(Gap(Value::from(8)))
                .with_style(Padding::left(10))
                .with_style(Padding::right(10))
                .with_handler::<TabRowReleased>(move |TabRowReleased(delta)| {
                    if *delta == 0.0 {
                        on_select(tab_id);
                    } else {
                        let positions = (delta / (ROW_HEIGHT + ROW_GAP)).round() as i32;
                        if positions != 0 {
                            on_reorder(tab_id, positions);
                        }
                    }
                })
                .with_slot::<TabFavicon>(move |favicon| favicon.with_style(Size::new(18)).with_element(FaviconImage { host: host.clone() }))
                .with_slot::<TabTitle>(move |title_slot| title_slot.with_style(Flex::new(1)).with_content(Some(title.clone())))
                .with_slot::<TabClose>(move |close| {
                    close.with_style(Size::width(22)).with_element(Button::new("\u{00D7}")).with_handler::<ClickEvent>(move |_| on_close(tab_id))
                })
        });
    }

    root.with_slot::<NewTabButton>(move |new_tab| {
        new_tab
            .with_style(Size::height(ROW_HEIGHT))
            .with_element(Button::new("+ New Tab"))
            .with_handler::<ClickEvent>(move |_| on_new_tab())
    })
}
