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
/// Margin around the whole strip's contents — also what keeps a rounded tab row (see
/// `TabRowView::new`'s own `cornerRadius`) from touching the sidebar's own edges.
pub const TAB_STRIP_PADDING: f64 = 6.0;

pub struct TabRow;
pub struct TabFavicon;
pub struct TabTitle;
pub struct TabClose;
pub struct NewTabButton;
/// A plain, contentless `Flex(1)` filler — used twice (before and after the favicon) in
/// `icon_only` mode to center it, since `escher_core::style` has no `JustifyContent`/`AlignItems`
/// property yet. Reused for both sides rather than two distinctly-named types: this module's
/// `NodePath` identity already keys on insertion order within the same parent (see the arena-slot
/// system's own doc comments elsewhere), so the same marker type twice in sequence still resolves
/// to two distinct, stable nodes.
pub struct TabIconSpacer;

/// Builds the tab strip's content onto `root`. `active` is the currently-selected tab's `id` (if
/// any), used only to highlight that row — see `crate::surface::TabRowMarker::selected`.
/// `icon_only` (see `TabStripState::icon_only`, true once the sidebar's been dragged narrower
/// than `ICON_ONLY_WIDTH`) drops the title/close-button slots and centers the favicon instead of
/// left-aligning it, same "drag it thin enough and it becomes an icon rail" convention a real
/// resizable sidebar needs. `on_select`/`on_close` fire with the clicked tab's `id`; `on_reorder`
/// fires with `(id, positions)` — `positions` is how many rows up (negative) or down (positive)
/// the drag ended relative to where it started, already converted from raw pixel displacement
/// using this module's own row geometry, so the caller just needs to move that tab `positions`
/// slots in its own list (clamped to bounds) rather than re-deriving it from pixels itself.
/// `on_new_tab` fires when the trailing "+" row is clicked.
pub fn tab_strip<'ctx>(
    root: Scaffold<'ctx>,
    tabs: &[TabInfo],
    active: Option<u64>,
    icon_only: bool,
    on_select: impl Fn(u64) + Clone + 'static,
    on_close: impl Fn(u64) + Clone + 'static,
    on_reorder: impl Fn(u64, i32) + Clone + 'static,
    on_new_tab: impl Fn() + 'static,
) -> Scaffold<'ctx> {
    let mut root = root.style(FlexDirection::Column).style(Gap(Value::from(ROW_GAP))).style(Padding::all(TAB_STRIP_PADDING as i32));

    for tab in tabs {
        let tab_id = tab.id;
        let title = tab.title.clone();
        let host = tab.host.clone();
        let on_select = on_select.clone();
        let on_close = on_close.clone();
        let on_reorder = on_reorder.clone();

        root = root.slot::<TabRow>(move |row| {
            let row = row
                .element(TabRowMarker { selected: Some(tab_id) == active })
                .style(FlexDirection::Row)
                .style(Size::height(ROW_HEIGHT))
                .style(Gap(Value::from(8)))
                .style(Padding::left(if icon_only { 0 } else { 10 }))
                .style(Padding::right(if icon_only { 0 } else { 10 }))
                // Vertically centers everything in the row at the favicon's own 18px height —
                // `ROW_HEIGHT` (38) minus this top/bottom pair (10 each) leaves exactly 18px of
                // content height, so the title/close button (no explicit cross-size of their own,
                // so they stretch to fill whatever's left) end up exactly favicon-height too,
                // instead of stretching to the row's full height and reading as vertically
                // misaligned against the icon next to them.
                .style(Padding::top(10))
                .style(Padding::bottom(10))
                .handle::<TabRowReleased>(move |TabRowReleased(delta)| {
                    if *delta == 0.0 {
                        on_select(tab_id);
                    } else {
                        let positions = (delta / (ROW_HEIGHT + ROW_GAP)).round() as i32;
                        if positions != 0 {
                            on_reorder(tab_id, positions);
                        }
                    }
                });

            if icon_only {
                // Centered rather than left-aligned, via a `Flex(1)` filler on each side — with no
                // title/close button sharing the row, a left-pinned favicon would read as
                // stranded against the row's own left edge instead of looking deliberately placed.
                row
                    .slot::<TabIconSpacer>(|spacer| {
                        spacer.style(Flex::new(1))
                    })
                    .slot::<TabFavicon>(move |favicon| {
                        favicon.style(Size::new(18)).element(FaviconImage { host: host.clone() })
                    })
                    .slot::<TabIconSpacer>(|spacer| {
                        spacer.style(Flex::new(1))
                    })
            } else {
                row
                    .slot::<TabFavicon>(move |favicon| {
                        favicon.style(Size::new(18)).element(FaviconImage { host: host.clone() })
                    })
                    .slot::<TabTitle>(move |title_slot| {
                        title_slot.style(Flex::new(1)).content(Some(title.clone()))
                    })
                    .slot::<TabClose>(move |close| {
                        close.style(Size::width(22)).element(Button::new("\u{00D7}")).handle::<ClickEvent>(move |_| on_close(tab_id))
                    })
            }
        });
    }

    root.slot::<NewTabButton>(move |new_tab| {
        new_tab
            .style(Size::height(ROW_HEIGHT))
            .element(Button::new(if icon_only { "+" } else { "+ New Tab" }))
            .handle::<ClickEvent>(move |_| on_new_tab())
    })
}
