#![allow(unused)]

use std::any::TypeId;
use std::fmt::Write;
use std::io::Error as IoError;
use std::io::Stderr;
use std::io::Stdin;
use std::io::Stdout;
use std::marker::PhantomData;
use std::time::Duration;

use derive_more::Deref;
use derive_more::DerefMut;

use bumpalo_herd::Herd;

use color_eyre::Result;

use num_traits::AsPrimitive;

use ansi_to_tui::IntoText;

use num_traits::Zero;

use ratatui::crossterm::event::KeyModifiers;
use ratatui::Frame;
use ratatui::Terminal;
use ratatui::backend::Backend;
use ratatui::backend::CrosstermBackend;
use ratatui::layout::Alignment;
use ratatui::layout::Constraint;
use ratatui::layout::Direction;
use ratatui::layout::Layout;
use ratatui::layout::Rect;
use ratatui::style::Style;
use ratatui::style::Stylize;
use ratatui::style::Color as RatatuiColor;
use ratatui::text::Line;
use ratatui::text::Text;
use ratatui::widgets;
use ratatui::widgets::Block;
use ratatui::widgets::Clear;
use ratatui::widgets::Padding;
use ratatui::widgets::Borders;
use ratatui::widgets::BorderType;
use ratatui::widgets::Paragraph;
use ratatui::widgets::Wrap;
use ratatui::widgets::Scrollbar;
use ratatui::widgets::ScrollbarOrientation;
use ratatui::widgets::ScrollbarState;
use ratatui::crossterm::event::Event as CrosstermEvent;
use ratatui::crossterm::event::KeyCode;
use ratatui::crossterm::event::KeyEvent;
use ratatui::crossterm::event::KeyEventKind;
use ratatui::crossterm::event::KeyEventState;
use ratatui::crossterm::event::MouseButton;
use ratatui::crossterm::event::MouseEvent;
use ratatui::crossterm::event::MouseEventKind;

use escher_core::event::keyboard::Code;
use escher_core::event::keyboard::Key;
use escher_core::event::keyboard::KeyState;
use escher_core::event::keyboard::KeyboardEvent;
use escher_core::event::keyboard::Location;
use escher_core::event::keyboard::Modifiers;
use escher_core::event::keyboard::NamedKey;
use escher_core::surface::Surface;
use escher_core::scaffold::Scaffold;
use escher_core::style::Unit;
use escher_core::style::Value;
use escher_core::style::Property;
use escher_core::style::Size;
use escher_core::style::FlexDirection;
use escher_core::style::Edge;
use escher_core::style::OverlayInset;
use escher_core::style::Gap;
use escher_core::style::Flex;
use escher_core::style::BackgroundColor;
use escher_core::style::ContentColor;
use escher_core::style::FontStyle;
use escher_core::style::FontWeight;
use escher_core::style::TextDecorationLine;
use escher_core::style::TextAlign;
use escher_core::style::Overflow;
use escher_core::style::Heading;
use escher_core::style::ScrollPosition;
use escher_core::style::Border;
use escher_core::content::LineCounter;
use escher_core::content::display_width;

use crate::app::TerminalAction;

//---
/// A mouse click at a specific terminal cell, deliberately not `ui_events::pointer`'s
/// `PointerButtonEvent`, which is built for continuous, sub-pixel pointer devices (pressure,
/// tilt, contact geometry, DPI scale), none of which have a meaningful value on a discrete
/// terminal cell grid. A widget registers interest via `.with_handler::<ClickEvent>(..)`, same as
/// any other event type.
#[derive(Debug, Clone, Copy)]
pub struct ClickEvent {
    pub column: u16,
    pub row: u16,
    pub button: MouseButton,
    pub modifiers: Modifiers,
}

/// A click-drag text selection, tracked across frames (unlike hit-testing, which only needs to
/// live for the duration of a single `draw()` call, a selection has to persist from the
/// mouse-down frame through however many drag frames until mouse-up). Coordinates are terminal
/// cells, `(column, row)`, matching crossterm's `MouseEvent` fields.
#[derive(Debug, Clone, Copy)]
struct Selection {
    anchor: (u16, u16),
    current: (u16, u16),
    /// The content-bearing node's own `Rect` the selection started in. Both points are kept
    /// clamped inside it, so a drag can't wander into chrome (borders, padding gutters, the
    /// header/footer/status line, another node's content) even if the mouse itself does.
    bounds: Rect,
}

impl Selection {
    /// Returns `(start, end)` in reading order (row-major: top-to-bottom, then left-to-right on
    /// a shared row) regardless of which direction the drag actually happened. This is a
    /// "stream" selection, the same model every normal terminal emulator uses, as opposed to a
    /// rectangular "block" selection where drag direction wouldn't matter this way.
    fn ordered(&self) -> ((u16, u16), (u16, u16)) {
        let reading_order = |point: (u16, u16)| (point.1, point.0); // (row, column)

        if reading_order(self.anchor) <= reading_order(self.current) {
            (self.anchor, self.current)
        } else {
            (self.current, self.anchor)
        }
    }
}

/// Which edge of a floating overlay a drag started on — whether the user grabbed the body
/// (move) or the resize handle in the bottom-right corner (resize). See `TerminalSurface::
/// overlay_drag_mode` for how a `Down` point is classified into one of these.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum OverlayDragMode {
    Move,
    Resize,
}

/// A click-drag move/resize of the floating overlay, tracked across frames the same way
/// `Selection` is (see its own doc comment) — a drag spans many `Drag` events between the
/// `Down` that starts it and the `Up` that ends it. Unlike `Selection`, this doesn't need an
/// `ordered()`-style helper: `anchor`/`start_bounds` are combined with the *current* mouse
/// point fresh on every `Drag`/`Up` event (see `TerminalSurface::resolve_overlay_drag`) rather
/// than needing to be read back out of the struct in reading order.
#[derive(Debug, Clone, Copy)]
struct OverlayDrag {
    mode: OverlayDragMode,
    /// Mouse position at `Down` — every subsequent `Drag`/`Up` point is compared against this
    /// to get a delta, rather than against the previous frame's point, so small per-frame
    /// rounding/clamping can never accumulate drift over a long drag.
    anchor: (u16, u16),
    /// The overlay's own `Rect` at the moment the drag started (already resolved — either a
    /// prior override or `overlay_rect`'s computed default). The delta from `anchor` is applied
    /// on top of *this*, not the previous frame's (possibly already-clamped) `Rect`, for the
    /// same drift-proofing reason as `anchor` itself.
    start_bounds: Rect,
}

#[derive(Deref, DerefMut)]
pub struct TerminalSurface<B: Backend> {
    #[deref]
    #[deref_mut]
    terminal: Terminal<B>,
    allocator: Herd,
    selection: Option<Selection>,
    /// The user's live drag/resize override for the overlay's position and size, `None` until
    /// they first interact with it (at which point it's seeded from whatever `overlay_rect`
    /// would have computed, so it doesn't jump — see `resolve_overlay_rect`), and persisting
    /// from then on for the rest of the session. There's deliberately no reset-to-default path
    /// yet (double-click, say) — out of scope for this pass.
    overlay_bounds: Option<Rect>,
    /// Only `Some` while a move/resize drag is actually in progress, between the `Down` that
    /// starts it and the `Up` that ends it (which clears it back to `None`) — unlike
    /// `overlay_bounds`, this has no reason to outlive the gesture itself.
    overlay_drag: Option<OverlayDrag>,
}

impl<B: Backend + core::fmt::Debug> core::fmt::Debug for TerminalSurface<B> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> core::fmt::Result {
        f.debug_struct("RatatuiSurface").field("terminal", &self.terminal).finish()
    }
}

impl<B: Backend> TerminalSurface<B> {
    pub fn new(backend: B) -> Result<Self>
    where
        <B as Backend>::Error: Send + Sync + 'static,
    {
        Ok(TerminalSurface {
            terminal: Terminal::<B>::new(backend)?,
            allocator: Herd::new(),
            selection: None,
            overlay_bounds: None,
            overlay_drag: None,
        })
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    pub fn try_default() -> Result<Self> {
        Ok(TerminalSurface {
            terminal: Terminal::new(CrosstermBackend::new(std::io::stdout()))?,
            allocator: Herd::new(),
            selection: None,
            overlay_bounds: None,
            overlay_drag: None,
        })
    }
}

impl<B: Backend> TerminalSurface<B> {
    /// The overlay's current live override, when the user has moved/resized it at least once
    /// this session — see `overlay_bounds`'s own field doc comment. A caller wanting to persist
    /// the overlay's position (across restarts, say) reads this each frame rather than reaching
    /// into `dispatch`/`handle_mouse_event` directly, since those are where it gets written but
    /// have no reason to know anything about persistence themselves.
    pub fn overlay_bounds(&self) -> Option<Rect> {
        self.overlay_bounds
    }

    /// Seeds the overlay's live override — meant to be called once, right after construction,
    /// with whatever a caller loaded from its own persistence layer, before the first `draw()`.
    /// Setting it later works too (there's nothing time-sensitive about it), it just won't be
    /// visible until the next frame, same as any other state written outside `draw()`.
    pub fn set_overlay_bounds(&mut self, bounds: Option<Rect>) {
        self.overlay_bounds = bounds;
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    pub fn stdout(&self) -> Stdout {
        std::io::stdout()
    }

    pub fn stderr(&self) -> Stderr {
        std::io::stderr()
    }

    pub fn stdin(&self) -> Stdin {
        std::io::stdin()
    }
}

impl Surface for TerminalSurface<CrosstermBackend<Stdout>> {
    type Event = TerminalAction;

    /// A 1-second poll meant the whole app only redrew once a second while idle (input arriving
    /// is the only other thing that triggers a redraw), any time-based animation (a spinner, a
    /// color pulse) was throttled to that same once-a-second cadence regardless of its own timing
    /// logic, since the frame computing its next state just wasn't being asked for that often.
    /// ~30fps keeps animation smooth without busy-looping. Only the right default for a caller
    /// that owns its own event loop and calls `draw` back-to-back in a tight `loop` (`app.rs`'s
    /// `TerminalApp::run`) — see `draw_with_poll_timeout`'s own doc comment for the Bevy-hosted
    /// case, where blocking here at all is actively wrong.
    fn draw<F>(&mut self, draw_scaffold_fn: F) -> Result<TerminalAction>
    where
        F: for<'ctx> FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>,
    {
        self.draw_with_poll_timeout(draw_scaffold_fn, Duration::from_millis(33))
    }
}

impl TerminalSurface<CrosstermBackend<Stdout>> {
    /// Same as `Surface::draw`, with the input-poll timeout exposed instead of hardcoded — a
    /// standalone event loop (`TerminalApp::run`) wants `draw` to block for a while so it isn't
    /// busy-looping, but a caller whose *own* scheduler already decides when to call this (an
    /// escher-bevy-hosted app, one call per Bevy tick) needs the opposite: blocking here at all
    /// stalls Bevy's entire main thread — rendering, animation, everything — for up to the full
    /// timeout on every tick that doesn't happen to have a terminal event already waiting, on top
    /// of whatever Bevy's own scheduling already decided this tick was worth running for. Pass
    /// `Duration::ZERO` in that case: check once for an already-pending event, dispatch it if
    /// there is one, and return immediately either way, letting Bevy's own wake-driven scheduling
    /// (window/device events, `spawn_input_watcher`) be the only thing that paces how often this
    /// gets called at all.
    pub fn draw_with_poll_timeout<F>(&mut self, draw_scaffold_fn: F, poll_timeout: Duration) -> Result<TerminalAction>
    where
        F: for<'ctx> FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>,
    {
        let arena = self.allocator.get();

        let scaffold = draw_scaffold_fn(Scaffold::new_in(arena.as_bump()));

        // TODO: Walk the scaffold and apply optimizations:
        //  - Unpack event handlers (where possible).
        //  - Build/validate/normalize styles.
        //  - Save a snapshot of each item.
        //
        // TODO: Optionally (via cfg) apply retained-mode rules:
        //  - Find and apply Node ids (lookup based on index + hash).
        //  - Nodes with changes should be marked.

        // Every rendered node's own `Rect`, in the order `render` visits them, used to answer
        // "which node is under this mouse position" after the frame draws. Lives only for this
        // one `draw()` call, rebuilt every frame since a stale Rect from a previous, different
        // layout would be worse than useless, unlike `self.selection` below, which has to
        // survive across frames since a drag spans many of them.
        let mut hit_regions = Vec::new();

        // A `Copy`d snapshot, not a borrow of `self.selection`. The closure below is passed to
        // `self.terminal.draw(..)`, which already needs `&mut self.terminal`. Capturing `self`
        // itself (rather than a plain local) would fight that borrow for no benefit, since the
        // closure only ever needs to *read* the current selection to paint its highlight.
        let selection_snapshot = self.selection;

        // Same reasoning as `selection_snapshot` — the closure only ever needs to *read* the
        // live drag/resize override to position the overlay this frame; any writes to it happen
        // later, in `dispatch`, once the frame (and this borrow of `self.terminal`) is done.
        let overlay_bounds_snapshot = self.overlay_bounds;

        let completed_frame = self.terminal.draw(|frame| {
            Self::render(&scaffold, frame, &frame.area(), &mut hit_regions);

            let overlay_rect = scaffold.get_overlay().map(|overlay| {
                let overlay_area = Self::resolve_overlay_rect(overlay_bounds_snapshot, overlay, &frame.area());
                Self::render(overlay, frame, &overlay_area, &mut hit_regions);
                Self::render_resize_handle(frame, overlay_area);
                overlay_area
            });

            if let Some(selection) = selection_snapshot {
                Self::render_selection_highlight(frame, &selection, overlay_rect);
            }
        })?;

        // Same computation as inside the draw closure above, just needed again out here for the
        // copy calls below and for mouse hit-testing — cheap and pure (only depends on the
        // scaffold, the frame area, and `self.overlay_bounds`, none of which the closure above
        // mutated), not worth threading a value out of the closure for.
        let overlay_rect = scaffold.get_overlay().map(|overlay| Self::resolve_overlay_rect(self.overlay_bounds, overlay, &completed_frame.area));

        #[cfg(all(feature="dev", feature="verbose"))]
        tracing::tracing!("Frame Area: {:?}", completed_frame.area);

        if crossterm::event::poll(poll_timeout)? {
            let event = ratatui::crossterm::event::read()?;

            // A selection is only meaningful to copy once it's finished (mouse-up), reading
            // the just-rendered `completed_frame.buffer` has to happen here, before `dispatch`,
            // since `dispatch` needs `&mut self` (to update `self.selection` for the next frame)
            // and `completed_frame` is still borrowing `self.terminal` at this point.
            if let CrosstermEvent::Mouse(MouseEvent { kind: MouseEventKind::Up(MouseButton::Left), .. }) = &event
                && let Some(selection) = self.selection
            {
                Self::copy_selection_to_clipboard(completed_frame.buffer, &selection, overlay_rect);
            }

            // Ctrl+C copies an active selection instead of being typed, same buffer-access
            // constraint as the mouse-up copy above, so it has to happen here too. Any *other*
            // keystroke while a selection exists clears it instead: a highlighted selection
            // lingering after the user's clearly moved on to typing again would be confusing,
            // and this is what "focus returns to the input" actually means here, since keyboard
            // input already goes straight to the input by default (see `dispatch`'s
            // `CrosstermEvent::Key` handling, every key not consumed above reaches the app's
            // own handler, which is what the assistant/ratatui examples wire the input box to).
            if let CrosstermEvent::Key(key_event) = &event
                && key_event.kind != ratatui::crossterm::event::KeyEventKind::Release
            {
                let is_copy_shortcut =
                    key_event.code == KeyCode::Char('c') && key_event.modifiers.contains(KeyModifiers::CONTROL);

                if is_copy_shortcut {
                    if let Some(selection) = self.selection {
                        Self::copy_selection_to_clipboard(completed_frame.buffer, &selection, overlay_rect);
                        return Ok(TerminalAction::Copied);
                    }
                    return Ok(TerminalAction::EmptyCopyAttempt);
                } else if self.selection.is_some() {
                    self.selection = None;
                }
            }

            return Self::dispatch(
                &scaffold,
                &event,
                &hit_regions,
                completed_frame.buffer,
                overlay_rect,
                completed_frame.area,
                &mut self.selection,
                &mut self.overlay_bounds,
                &mut self.overlay_drag,
            );
        }

        Ok(TerminalAction::NoOp)
    }
}

/// The bottom-right resize grip's size, in cells — small and unobtrusive, matching the couple-
/// of-cells convention normal terminal window managers (tmux/iTerm2 pane corners) use for the
/// same purpose, not a large, obvious handle that would eat into the overlay's own content area.
const RESIZE_HANDLE_SIZE: u16 = 2;

/// The smallest the overlay can be resized down to — small enough to still comfortably show a
/// couple of characters of a task label, too small to become a degenerate sliver that's fiddly
/// to grab or resize back up from.
const OVERLAY_MIN_WIDTH: u16 = 12;
const OVERLAY_MIN_HEIGHT: u16 = 3;

impl TerminalSurface<CrosstermBackend<Stdout>> {
    /// Positions an overlay in the bottom-right corner of `frame_bounds`, sized from its own
    /// `Size` style (falling back to a sensible default), inset by one cell on each edge so it
    /// reads as a floating window rather than being flush against the terminal's edge.
    ///
    /// The root layout (Header/Body/Footer, or whatever else a caller builds) isn't visible to
    /// this function, so an overlay that needs to clear chrome the layout owns (a footer bar,
    /// say) should set its own `OverlayInset` on the relevant edge(s). It's added on top of
    /// the fixed 1-cell inset. `Edge::Bottom`/`Edge::Right`/`Edge::All` are honored. `Top`/`Left`
    /// don't apply to a corner that's always anchored bottom-right.
    fn overlay_rect(overlay: &Scaffold, frame_bounds: &Rect) -> Rect {
        const MARGIN: u16 = 1;
        const DEFAULT_WIDTH: u16 = 30;
        const DEFAULT_HEIGHT: u16 = 6;

        let mut width = DEFAULT_WIDTH;
        let mut height = DEFAULT_HEIGHT;
        let mut margin_right = MARGIN;
        let mut margin_bottom = MARGIN;

        if let Some(sizes) = overlay.get_styles().get(&TypeId::of::<Size>()) {
            for property in sizes.iter() {
                if let Property::Size(Size(Value::Px(w), Value::Px(h), ..)) = property {
                    width = <Unit as AsPrimitive<u16>>::as_(*w);
                    height = <Unit as AsPrimitive<u16>>::as_(*h);
                }
            }
        }

        if let Some(insets) = overlay.get_styles().get(&TypeId::of::<OverlayInset>()) {
            for property in insets.iter() {
                if let Property::OverlayInset(OverlayInset(edge, value)) = property {
                    let extra = <Value as AsPrimitive<u16>>::as_(*value);
                    match edge {
                        Edge::Bottom => margin_bottom += extra,
                        Edge::Right => margin_right += extra,
                        Edge::All => {
                            margin_bottom += extra;
                            margin_right += extra;
                        }
                        _ => {}
                    }
                }
            }
        }

        width = width.min(frame_bounds.width.saturating_sub(margin_right + MARGIN));
        height = height.min(frame_bounds.height.saturating_sub(margin_bottom + MARGIN));

        Rect {
            x: frame_bounds.x + frame_bounds.width.saturating_sub(width + margin_right),
            y: frame_bounds.y + frame_bounds.height.saturating_sub(height + margin_bottom),
            width,
            height,
        }
    }

    /// The overlay's actual `Rect` for this frame: `overlay_bounds`, the user's own live
    /// drag/resize override, when they've set one, otherwise `overlay_rect`'s computed
    /// bottom-right-anchored default (which is also exactly what the override gets seeded from
    /// the moment a drag/resize starts — see `OverlayDrag`/`handle_mouse_event` — so the switch
    /// from default to override is never a visible jump). Both `draw()` call sites that used to
    /// call `overlay_rect` directly call this instead, so rendering, hit-testing, and the
    /// selection/clipboard exclusion logic all agree on where the overlay actually is.
    fn resolve_overlay_rect(overlay_bounds: Option<Rect>, overlay: &Scaffold, frame_bounds: &Rect) -> Rect {
        overlay_bounds.unwrap_or_else(|| Self::overlay_rect(overlay, frame_bounds))
    }

    fn render<'a, 'ctx>(scaffold: &'a Scaffold<'ctx>, frame: &mut Frame, frame_bounds: &Rect, hit_regions: &mut Vec<(Rect, &'a Scaffold<'ctx>)>) {
        if !scaffold.is_enabled() {
            return;
        }
        
        let mut content_bounds = *frame_bounds;
        let mut border_size = (0u16, 0u16);

        if scaffold.is_debug() {
            // TODO: Draw debug information on this!
            let debug_block = Block::default()
                .title_top({
                    let Rect { x, y, .. } = content_bounds;
                    Line::from(format!("Pos: {}x{}", x, y))
                        .alignment(Alignment::Left)
                })
                .title_bottom({
                    let Rect { width, height, .. } = content_bounds;
                    Line::from(format!("Size: {}x{}", width, height))
                        .alignment(Alignment::Right)
                })
                .border_style(Style::new().dim())
                .border_type(BorderType::Rounded)
                .borders(Borders::ALL);
            
            let xray_inner_area = debug_block.inner(content_bounds);
            frame.render_widget(debug_block, *frame_bounds);
            
            content_bounds = xray_inner_area;
            
            border_size.0 += 1;
            border_size.1 += 1;
        }

        if let Some(borders) = scaffold.get_styles().get(&TypeId::of::<Border>()) {
            for border in borders.iter() {
                if let Property::Border(Border(border_edge, border_value, border_style, border_color)) = border && !border_value.is_zero() {
                    let mut border_line_style = Style::new();
                    if let Some(color) = border_color.map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue)) {
                        border_line_style = border_line_style.fg(color);
                    }

                    // `Block::border_style` only colors the border glyphs themselves. It was
                    // the only style ever applied here, which meant a node with both `Border`
                    // and `BackgroundColor` got its background painted under the *content* (via
                    // the Paragraph's own style, further down) but never under the border ring.
                    // Whatever had already been drawn there earlier in the frame (e.g. content
                    // from a sibling this node overlaps, like an overlay) showed through at
                    // exactly the border cells instead. `Block::style` fills the *entire* block
                    // area, border and interior alike, so it needs the background too.
                    let mut fill_style = Style::new();
                    if let Some(bg) = scaffold.get_styles().get(&TypeId::of::<BackgroundColor>()).and_then(|values| {
                        values.iter().find_map(|value| match value {
                            Property::BackgroundColor(color) => {
                                color.map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue))
                            }
                            _ => None,
                        })
                    }) {
                        fill_style = fill_style.bg(bg);
                    }

                    // `Block::render`'s own background fill (`buf.set_style`, immediately
                    // below) only ever touches a cell's *color*, never its character (see
                    // `ratatui-core`'s `Cell::set_style`, which sets `fg`/`bg` and nothing
                    // else). A cell this block's own border/content drawing never writes a new
                    // character into keeps whatever glyph was left there earlier this frame or
                    // a previous one. The padding gap between the border and the padding-inset
                    // content area is exactly such a cell, nothing pads out that far, so a
                    // sibling this node overlaps can show stale content through it (an
                    // overlay's padding gap showing stale transcript text is exactly the bug
                    // this fixes). `Clear` resets both the character and the style for every
                    // cell in its area, so nothing can survive underneath it. This is the
                    // standard ratatui pattern for a popup redrawing over whatever's there,
                    // worth doing only when there's a background to protect in the first
                    // place, since a transparent bordered box legitimately wants to show
                    // through.
                    if fill_style.bg.is_some() {
                        frame.render_widget(Clear, content_bounds);
                    }

                    let border_block = Block::default()
                        // .title_top(Line::from("TODO: Border labels ..").alignment(Alignment::Left))
                        // TODO: Use the border_weight to determine
                        .style(fill_style)
                        .border_style(border_line_style)
                        .border_type(BorderType::Rounded)
                        .borders(Borders::ALL);
                    let border_inner_area = border_block.inner(content_bounds);
                    frame.render_widget(border_block, content_bounds);
                    content_bounds = border_inner_area;
                    border_size.0 += 1;
                    border_size.1 += 1;
                }
            }
        }

        //--
        let flex_direction = scaffold
            .get_styles()
            .iter()
            .find(|(type_id, values)| **type_id == TypeId::of::<FlexDirection>())
            .map(|(_, values)| match values.first() {
                Some(Property::FlexDirection(direction)) => *direction,
                _ => FlexDirection::Column,
            })
            .unwrap_or_default();

        // Margin insets the area available to this node's own content and to its children.
        // Escher doesn't distinguish "space around this box" from "space before its children"
        // the way CSS does, so one inset serves both. Multiple `Margin` entries (e.g. separate
        // `Margin::top(..)` and `Margin::left(..)` calls) accumulate per edge.
        let margin = sum_insets(scaffold.get_styles().iter().flat_map(|style| style.1).filter_map(|property| match property {
            Property::Margin(margin) => Some(into_margin(margin)),
            _ => None,
        }));

        content_bounds = Rect {
            x: content_bounds.x + margin.left,
            y: content_bounds.y + margin.top,
            width: content_bounds.width.saturating_sub(margin.left + margin.right),
            height: content_bounds.height.saturating_sub(margin.top + margin.bottom),
        };

        // Recorded regardless of whether this node has any handlers, cheap (a `Rect` copy and
        // a reference), and simpler than threading a "does this subtree have any handlers"
        // check through every style/layout branch above just to skip a push here and there.
        hit_regions.push((content_bounds, scaffold));

        // Padding insets specifically this node's own content text (between the border, already
        // applied above, and the text itself), computed up front so the `LineCounter` wrap
        // width matches the rect the `Paragraph` actually ends up drawn into.
        let padding = sum_insets(scaffold.get_styles().iter().flat_map(|style| style.1).filter_map(|property| match property {
            Property::Padding(padding) => Some(into_padding(padding)),
            _ => None,
        }));

        let mut layout_constraints = Vec::with_capacity(scaffold.get_slots().len() + 1);

        if let Some(content) = scaffold.get_content() {
            let wrap_width = content_bounds.width.saturating_sub(padding.left + padding.right);
            let mut lines = LineCounter::<u16>::new(wrap_width as usize);

            if let Err(error) = write!(&mut lines, "{}", content) {
                tracing::error!("Failed to count content lines: {}", error)
            }

            layout_constraints.push(match flex_direction {
                FlexDirection::Column => Constraint::Length(lines.count() + padding.top + padding.bottom),
                FlexDirection::Row => Constraint::Fill(1),
            });
        }

        //--
        for (_, slot) in scaffold.get_slots().iter() {
            if slot.is_enabled() {
                layout_constraints.push({
                    slot.get_styles()
                        .get(&TypeId::of::<Size>())
                        .and_then(|values| {
                            values
                                .iter()
                                .find(|value| matches!(value, Property::Size(..)))
                                .and_then(|value| match value {
                                    Property::Size(Size(Value::Px(width), ..)) if flex_direction.is_row() => {
                                        Some(Constraint::Length(<Unit as AsPrimitive<u16>>::as_(*width)))
                                    }
                                    Property::Size(Size(_, Value::Px(height), ..)) if flex_direction.is_column() => {
                                        Some(Constraint::Length(<Unit as AsPrimitive<u16>>::as_(*height)))
                                    }
                                    _ => None
                                })
                        })
                        // No explicit Size, fall back to a declared Flex grow weight (like
                        // CSS `flex-grow`) before defaulting to an even Min(0) share.
                        .or_else(|| {
                            slot.get_styles()
                                .get(&TypeId::of::<Flex>())
                                .and_then(|values| values.iter().find_map(|value| match value {
                                    Property::Flex(Flex(weight)) => {
                                        Some(Constraint::Fill(<Unit as AsPrimitive<u16>>::as_(*weight).max(1)))
                                    }
                                    _ => None,
                                }))
                        })
                        .unwrap_or(Constraint::Min(0))
                });
            }
        }

        //---
        let mut layout = Layout::new(into_direction(flex_direction), layout_constraints);

        for property in scaffold.get_styles().iter().flat_map(|style| style.1) {
            if let Property::Gap(Gap(gap)) = property {
                layout = layout.spacing::<u16>(gap.as_())
            }
        }

        //---
        let layout_areas = layout.split(content_bounds);
        let mut slot_areas = layout_areas.into_iter();
        let content_area = scaffold.get_content().and_then(|_| slot_areas.next());

        if let Some(content_area) = content_area
        && let Some(content) = scaffold.get_content() {
            let content_area = Rect {
                x: content_area.x + padding.left,
                y: content_area.y + padding.top,
                width: content_area.width.saturating_sub(padding.left + padding.right),
                height: content_area.height.saturating_sub(padding.top + padding.bottom),
            };

            let mut style = Style::default();
            let mut alignment = Alignment::Left;
            let mut overflow = Overflow::default();
            let mut scroll_offset = 0u16;

            for property in scaffold.get_styles().iter().flat_map(|style| style.1) {
                match property {
                    Property::FontStyle(font_style) => {
                        style = match font_style {
                            FontStyle::Normal => style.not_italic(),
                            FontStyle::Italic => style.italic(),
                        }
                    }
                    Property::FontWeight(weight) => {
                        style = match weight {
                            FontWeight::Normal => style.not_bold(),
                            FontWeight::Bold => style.bold(),
                        }
                    }
                    Property::TextDecorationLine(decoration) => {
                        style = match decoration {
                            TextDecorationLine::None => style.not_underlined().not_crossed_out(),
                            TextDecorationLine::Underline => style.underlined(),
                            TextDecorationLine::LineThrough => style.crossed_out(),
                        }
                    }
                    Property::TextAlign(text_align) => {
                        alignment = match text_align {
                            TextAlign::Left => Alignment::Left,
                            TextAlign::Center => Alignment::Center,
                            TextAlign::Right => Alignment::Right,
                        };
                    }
                    Property::Heading(_heading) => {
                        // Terminal cells are fixed-size, there's no `font-size` scale to give
                        // each heading level a distinct visual size, so every variant just
                        // reads as emphasized (bold) text.
                        style = style.bold();
                    }
                    Property::Overflow(value) => {
                        overflow = *value;
                    }
                    Property::ScrollPosition(ScrollPosition(offset)) => {
                        scroll_offset = <Unit as AsPrimitive<u16>>::as_(*offset);
                    }
                    Property::BackgroundColor(color) => {
                        style = style.bg({
                            color
                                .map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue))
                                .unwrap_or_default()
                        });
                    }
                    Property::ContentColor(color) => {
                        style = style.fg({
                            color
                                .map(|color| RatatuiColor::Rgb(color.red, color.green, color.blue))
                                .unwrap_or_default()
                        });
                    }
                    _ => continue
                }
            }

            // `Paragraph` only paints a background under a line's *own* text. Cells beyond
            // the end of a shorter line are left completely untouched, exposing whatever was
            // drawn underneath earlier in the same frame (a sibling this node overlaps, e.g.
            // an overlay over the main content). Pad every line out to the full width with
            // plain spaces so a background actually fills the whole box, but only when
            // there's a background to fill since padding every frame for no visual reason is
            // wasted work.
            let content_text = content.to_string();
            let content_text = if style.bg.is_some() {
                pad_lines_to_width(&content_text, content_area.width as usize)
            } else {
                content_text
            };

            // `LineCounter` (see `escher_core::content`) computes row height on the assumption
            // that content word-wraps at the available width, so the Paragraph must too.
            let widget = match content_text.into_text() {
                Ok(mut text) => {
                    // `ansi_to_tui` parses a full ANSI reset (`\x1b[0m`, what `owo_colors`
                    // emits after every styled segment, used throughout this app's content for
                    // role/diff/status coloring) into an *explicit* `Color::Reset` background on
                    // that span, not "no override, inherit from below". Ratatui then paints
                    // `Color::Reset` as a real value (a literal background-reset SGR code),
                    // clobbering whatever this node's own `BackgroundColor` set. Invisible
                    // until content combines embedded ANSI coloring *with* a node-level
                    // `BackgroundColor` (the tasks/autocomplete overlay is the only place that
                    // combination shows up so far, confirmed by inspecting the raw SGR bytes
                    // ratatui actually emits via `tmux capture-pane -e`, not just visually).
                    // Only needs fixing where there's a background to protect in the first
                    // place.
                    if let Some(background) = style.bg {
                        for line in text.lines.iter_mut() {
                            for span in line.spans.iter_mut() {
                                if matches!(span.style.bg, None | Some(RatatuiColor::Reset)) {
                                    span.style.bg = Some(background);
                                }
                            }
                        }
                    }

                    let mut widget = Paragraph::new(text).style(style).alignment(alignment).wrap(Wrap { trim: false });
                    // A scroll offset only takes effect once a node opts into `Overflow::Scroll`.
                    // Otherwise a stray `ScrollPosition` (e.g. left over from a previous UI
                    // state) can't silently start clipping content that never asked to scroll.
                    if overflow.is_scroll() {
                        widget = widget.scroll((scroll_offset, 0));
                    }
                    widget
                }
                Err(error) => {
                    tracing::error!("Failed to get rich text from content: {}", error);
                    Paragraph::new("ERROR".red())
                }
            };

            frame.render_widget(widget, content_area);

            // A dynamically-sized scrollbar tracking `scroll_offset`. Only drawn once there's
            // actually more content than fits, so it doesn't appear on content that happens to
            // have `Overflow::Scroll` set but isn't currently scrollable.
            if overflow.is_scroll() {
                let mut total_height_counter = LineCounter::<u16>::new(content_area.width as usize);
                let _ = write!(&mut total_height_counter, "{}", content);
                let total_height = total_height_counter.count();

                if total_height > content_area.height {
                    // `ScrollbarState::position` is expected by ratatui to range up to
                    // `content_length - 1` at maximum scroll (its canonical use is a list
                    // selection index, not a row offset), but our `scroll_offset` is clamped to
                    // `total_height - viewport_height` so the last row lands flush at the
                    // viewport's bottom rather than leaving trailing blank space, which is
                    // *less* than `total_height - 1` whenever the viewport is more than one row
                    // tall. Feeding the raw `total_height` in as `content_length` therefore left
                    // the thumb short of the track's end even at full scroll. Using our own
                    // actual max position (`max_scroll_offset + 1`) as `content_length` instead
                    // makes both the thumb's position and its size correct. It also happens to
                    // make `max_scroll_offset + viewport_height` equal `total_height` again,
                    // which is what makes the thumb size come out proportional to the true
                    // visible fraction of the content.
                    let max_scroll_offset = total_height.saturating_sub(content_area.height);
                    let mut scrollbar_state = ScrollbarState::new(max_scroll_offset as usize + 1)
                        .position(scroll_offset as usize)
                        .viewport_content_length(content_area.height as usize);

                    // A thin, dim thumb with no track line and no arrow glyphs. The default
                    // (a solid block thumb, a double-line track, and ▲/▼ arrows) reads as a
                    // prominent, separate UI element. This reads as a quiet position hint that's
                    // easy to ignore when you don't need it.
                    let scrollbar = Scrollbar::new(ScrollbarOrientation::VerticalRight)
                        .begin_symbol(None)
                        .end_symbol(None)
                        .track_symbol(None)
                        .thumb_symbol("│")
                        .thumb_style(Style::new().fg(RatatuiColor::DarkGray));

                    frame.render_stateful_widget(scrollbar, content_area, &mut scrollbar_state);
                }
            }
        }

        for (i, slot_area) in slot_areas.enumerate() {
            match scaffold.get_slots().values().nth(i) {
                Some(slot) => Self::render(slot, frame, slot_area, hit_regions),
                None => tracing::warn!("Failed to get slot {}!", i),
            }
        }
    }
    
    fn dispatch<'a, 'ctx>(
        scaffold: &Scaffold,
        event: &CrosstermEvent,
        hit_regions: &[(Rect, &'a Scaffold<'ctx>)],
        buffer: &ratatui::buffer::Buffer,
        overlay_rect: Option<Rect>,
        frame_bounds: Rect,
        selection: &mut Option<Selection>,
        overlay_bounds: &mut Option<Rect>,
        overlay_drag: &mut Option<OverlayDrag>,
    ) -> Result<TerminalAction> {
        match event {
            CrosstermEvent::FocusGained => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::FocusLost => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::Key(event) => {
                let event = unpack_keyboard_event(event);

                // TODO: This should be optional ..
                // `KeyState::Up` (a key *release* event) requires the terminal to opt into
                // the Kitty keyboard protocol, which this app never enables, so on ordinary
                // terminals no Release event is ever reported and this could never fire.
                // `Down` covers the initial Press, which every terminal reports.
                if event.code == Code::Escape && event.state == KeyState::Down {
                    return Ok(TerminalAction::Exit(0));
                }

                scaffold.dispatch::<KeyboardEvent>(&event);
            }
            CrosstermEvent::Mouse(mouse_event) => {
                Self::handle_mouse_event(
                    mouse_event,
                    hit_regions,
                    buffer,
                    overlay_rect,
                    frame_bounds,
                    selection,
                    overlay_bounds,
                    overlay_drag,
                );
            }
            CrosstermEvent::Paste(_) => {
                // tracing::debug!("TODO: Handle events!");
            }
            CrosstermEvent::Resize(_, _) => {
                // tracing::debug!("TODO: Handle events!");
            }
            unhandled_event => {
                #[cfg(all(feature="dev", feature="verbose"))]
                tracing::warn!("Surface Event: {:?}", unhandled_event);
            }
        }

        scaffold.dispatch::<CrosstermEvent>(event)?;

        Ok(TerminalAction::NoOp)
    }

    /// Left-button down first checks whether it landed on the overlay (via `overlay_drag_mode`)
    /// — if so, it starts a move/resize drag instead of a text selection, so grabbing the
    /// floating window's body or corner never also highlights its own text underneath the drag.
    /// Otherwise it starts a selection only if it landed on a content-bearing node (text, not
    /// chrome, see `hit_test_content`); anything else clears whatever selection already existed,
    /// same as clicking empty space to deselect in a normal terminal. The selection's bounds are
    /// narrowed from the whole node down to just the paragraph the click landed in (see
    /// `group_bounds`), so a drag can't wander into a sibling paragraph in the same node any more
    /// than it can wander into a different node entirely. Landing on a node with a registered
    /// `ClickEvent` handler fires it either way — selection, overlay drag, or neither — matching
    /// the "a plain click still reaches its handler, only an actual drag means something else"
    /// principle a click on the overlay follows too (nothing moves until a `Drag` event actually
    /// arrives; a `Down` immediately followed by `Up` at the same point never does). Drag/up
    /// extend whichever of selection/overlay-drag is active, clamped to their own bounds. The
    /// clipboard copy itself happens back in `draw()`, where the just-rendered buffer is still
    /// reachable. By the time control gets here that borrow is already gone.
    fn handle_mouse_event<'a, 'ctx>(
        mouse_event: &MouseEvent,
        hit_regions: &[(Rect, &'a Scaffold<'ctx>)],
        buffer: &ratatui::buffer::Buffer,
        overlay_rect: Option<Rect>,
        frame_bounds: Rect,
        selection: &mut Option<Selection>,
        overlay_bounds: &mut Option<Rect>,
        overlay_drag: &mut Option<OverlayDrag>,
    ) {
        let point = (mouse_event.column, mouse_event.row);

        match mouse_event.kind {
            MouseEventKind::Down(button) => {
                if button == MouseButton::Left {
                    let drag_mode = overlay_rect.and_then(|bounds| Self::overlay_drag_mode(bounds, point));

                    if let (Some(mode), Some(bounds)) = (drag_mode, overlay_rect) {
                        *overlay_drag = Some(OverlayDrag { mode, anchor: point, start_bounds: bounds });
                        // A drag that starts on the overlay itself is window chrome, not a text
                        // selection over its content — without this, the same click would also
                        // start selecting the overlay's own text underneath the move/resize.
                        *selection = None;
                    } else {
                        *overlay_drag = None;

                        // The overlay is excluded from group-boundary detection too, unless the
                        // click actually landed on the overlay itself (its own glyphs shouldn't
                        // make an otherwise-blank transcript row look non-blank, but the
                        // overlay's own content is real content when that's what was clicked) —
                        // moot here in practice since a click on the overlay always takes the
                        // `drag_mode` branch above instead, but kept for the same reasoning
                        // `group_bounds`'s `exclude` parameter documents elsewhere.
                        let exclude = overlay_rect.filter(|area| !rect_contains(*area, point.0, point.1));

                        *selection = Self::hit_test_content(hit_regions, point)
                            .map(|node_bounds| {
                                let bounds = Self::group_bounds(buffer, node_bounds, point.1, exclude);
                                Selection { anchor: point, current: point, bounds }
                            });
                    }
                }

                if let Some(hit) = Self::hit_test(hit_regions, point) {
                    let click = ClickEvent { column: point.0, row: point.1, button, modifiers: unpack_modifiers(&mouse_event.modifiers) };
                    hit.get_handlers().exec::<ClickEvent>(&click);
                }
            }
            MouseEventKind::Drag(MouseButton::Left) => {
                if let Some(drag) = overlay_drag.as_ref() {
                    *overlay_bounds = Some(Self::resolve_overlay_drag(drag, point, frame_bounds));
                } else if let Some(selection) = selection.as_mut() {
                    selection.current = clamp_to_rect(point, selection.bounds);
                }
            }
            MouseEventKind::Up(MouseButton::Left) => {
                if let Some(drag) = overlay_drag.take() {
                    *overlay_bounds = Some(Self::resolve_overlay_drag(&drag, point, frame_bounds));
                } else if let Some(selection) = selection.as_mut() {
                    selection.current = clamp_to_rect(point, selection.bounds);
                }
            }
            _ => {}
        }
    }

    /// Classifies a `Down` point that's landed inside the overlay's own `bounds` as either a
    /// resize (the bottom-right corner, `RESIZE_HANDLE_SIZE` cells square — matching the small,
    /// unobtrusive grip convention of a normal terminal window manager, e.g. tmux/iTerm2's own
    /// pane-resize corners) or a move (anywhere else in the overlay). Returns `None` when
    /// `point` isn't inside `bounds` at all — not the overlay's concern. `RESIZE_HANDLE_SIZE` is
    /// clamped down to whatever's smaller than the overlay's own current size so a
    /// `OVERLAY_MIN_WIDTH`/`OVERLAY_MIN_HEIGHT`-sized overlay doesn't end up entirely a resize
    /// handle with nowhere left to grab for a move.
    fn overlay_drag_mode(bounds: Rect, point: (u16, u16)) -> Option<OverlayDragMode> {
        if !rect_contains(bounds, point.0, point.1) {
            return None;
        }

        let handle_width = RESIZE_HANDLE_SIZE.min(bounds.width);
        let handle_height = RESIZE_HANDLE_SIZE.min(bounds.height);
        let handle_x = bounds.x + bounds.width - handle_width;
        let handle_y = bounds.y + bounds.height - handle_height;

        if point.0 >= handle_x && point.1 >= handle_y {
            Some(OverlayDragMode::Resize)
        } else {
            Some(OverlayDragMode::Move)
        }
    }

    /// Marks the resize handle with a distinct corner glyph so it's actually discoverable —
    /// without this, the handle is a real hit-region (`overlay_drag_mode` above) that's visually
    /// indistinguishable from the rest of the rounded border, nothing hints that specifically
    /// the bottom-right corner (as opposed to the rest of the overlay's edge) drags differently.
    /// Only the single corner-most cell is swapped, not the whole `RESIZE_HANDLE_SIZE` region —
    /// recoloring every cell in the hit-region would dim a visible stretch of the border for a
    /// grip that only needs one glyph to read as intentional, and the hit-test itself already
    /// uses the wider region so the visual can stay minimal without shrinking the grabbable
    /// area. Drawn after `render` so it always wins over whatever glyph the border would have
    /// put there.
    fn render_resize_handle(frame: &mut Frame, bounds: Rect) {
        if bounds.width == 0 || bounds.height == 0 {
            return;
        }

        let corner = (bounds.x + bounds.width - 1, bounds.y + bounds.height - 1);

        if let Some(cell) = frame.buffer_mut().cell_mut(corner) {
            cell.set_symbol("◢").set_style(Style::new().fg(RatatuiColor::DarkGray));
        }
    }

    /// Applies an in-progress move/resize drag's delta (the current `point` minus `drag.anchor`)
    /// on top of `drag.start_bounds`, clamped fully on-screen within `frame_bounds`: a move can't
    /// push any edge past the frame's own edges, and a resize can't shrink past
    /// `OVERLAY_MIN_WIDTH`/`OVERLAY_MIN_HEIGHT` or grow past whatever room is actually left
    /// between the overlay's fixed top-left corner and the frame's far edge. Computing every
    /// frame from `drag.anchor`/`drag.start_bounds` rather than incrementally off the previous
    /// frame's (already-clamped) `Rect` means clamping can never compound drift over a long drag
    /// — the same reasoning `OverlayDrag`'s own doc comment gives for storing those fields in
    /// the first place.
    fn resolve_overlay_drag(drag: &OverlayDrag, point: (u16, u16), frame_bounds: Rect) -> Rect {
        let dx = point.0 as i32 - drag.anchor.0 as i32;
        let dy = point.1 as i32 - drag.anchor.1 as i32;
        let start = drag.start_bounds;

        let frame_left = frame_bounds.x as i32;
        let frame_top = frame_bounds.y as i32;
        let frame_right = frame_left + frame_bounds.width as i32;
        let frame_bottom = frame_top + frame_bounds.height as i32;

        match drag.mode {
            OverlayDragMode::Move => {
                let max_x = (frame_right - start.width as i32).max(frame_left);
                let max_y = (frame_bottom - start.height as i32).max(frame_top);
                let x = (start.x as i32 + dx).clamp(frame_left, max_x);
                let y = (start.y as i32 + dy).clamp(frame_top, max_y);

                Rect { x: x as u16, y: y as u16, width: start.width, height: start.height }
            }
            OverlayDragMode::Resize => {
                let max_width = (frame_right - start.x as i32).max(OVERLAY_MIN_WIDTH as i32);
                let max_height = (frame_bottom - start.y as i32).max(OVERLAY_MIN_HEIGHT as i32);
                let width = (start.width as i32 + dx).clamp(OVERLAY_MIN_WIDTH as i32, max_width);
                let height = (start.height as i32 + dy).clamp(OVERLAY_MIN_HEIGHT as i32, max_height);

                Rect { x: start.x, y: start.y, width: width as u16, height: height as u16 }
            }
        }
    }

    /// The most specific (deepest) node whose own `Rect` contains `point`. `hit_regions` is
    /// populated in the same top-down order `render` visits nodes, so a child's entry always
    /// comes after its parent's. Searching in reverse and taking the first match finds the
    /// deepest one without needing to track parent/child relationships explicitly.
    fn hit_test<'a, 'ctx>(hit_regions: &[(Rect, &'a Scaffold<'ctx>)], point: (u16, u16)) -> Option<&'a Scaffold<'ctx>> {
        hit_regions
            .iter()
            .rev()
            .find(|(rect, _)| {
                point.0 >= rect.x && point.0 < rect.x + rect.width && point.1 >= rect.y && point.1 < rect.y + rect.height
            })
            .map(|(_, scaffold)| *scaffold)
    }

    /// Same as `hit_test`, restricted to nodes that actually carry text (`get_content()`),
    /// which is what a drag-select is allowed to start in and stay within. A border, a padding gutter, or
    /// an empty background area between two content blocks all show up in `hit_regions` (every
    /// node is recorded, see `render`) but none of them are selectable text, which is exactly
    /// the "selecting any string of terminal lines" bug this exists to fix.
    fn hit_test_content<'a, 'ctx>(hit_regions: &[(Rect, &'a Scaffold<'ctx>)], point: (u16, u16)) -> Option<Rect> {
        hit_regions
            .iter()
            .rev()
            .find(|(rect, scaffold)| {
                scaffold.get_content().is_some()
                    && point.0 >= rect.x && point.0 < rect.x + rect.width
                    && point.1 >= rect.y && point.1 < rect.y + rect.height
            })
            .map(|(rect, _)| *rect)
    }

    /// Narrows `node_bounds` down to the contiguous run of non-blank rows containing
    /// `anchor_row`, the actual paragraph a selection started in, not the whole node. A single
    /// node's content can hold several logical blocks joined by a blank line (e.g. the
    /// transcript, one message per block) — a row counts as blank when every cell across
    /// `node_bounds`'s columns holds no visible character, exactly what that blank-line join
    /// renders as. Falls back to the full `node_bounds` in either direction once no blank row
    /// bounds the group, so content with no internal blank-line breaks is unaffected. `exclude`
    /// is ignored when judging blankness, so an overlay drawn over an otherwise-blank row can't
    /// hide the paragraph break underneath it.
    fn group_bounds(buffer: &ratatui::buffer::Buffer, node_bounds: Rect, anchor_row: u16, exclude: Option<Rect>) -> Rect {
        let is_blank_row = |row: u16| -> bool {
            (node_bounds.x..node_bounds.x + node_bounds.width).all(|column| {
                if exclude.is_some_and(|area| rect_contains(area, column, row)) {
                    return true;
                }
                buffer.cell((column, row)).map(|cell| cell.symbol().trim().is_empty()).unwrap_or(true)
            })
        };

        let top = node_bounds.y;
        let bottom = node_bounds.y + node_bounds.height;

        let mut start = anchor_row;
        while start > top && !is_blank_row(start - 1) {
            start -= 1;
        }

        let mut end = anchor_row + 1;
        while end < bottom && !is_blank_row(end) {
            end += 1;
        }

        Rect { x: node_bounds.x, y: start, width: node_bounds.width, height: end - start }
    }

    /// The rightmost column in `[start_col, max_col]` on `row` that actually holds non-blank
    /// rendered content. A row's real text rarely fills the whole content node width (word-wrap
    /// leaves trailing blank cells after a short line), so selecting/copying the full row width
    /// grabs blank space that was never really text. Interior blanks (the space between two
    /// words) don't get trimmed, only the run of blank cells after the true last character does.
    /// `exclude`, when given, is a region (an overlay's own area) to skip entirely, so an
    /// overlay drawn on top of this row can't be mistaken for this row's own real content.
    fn content_end_column(buffer: &ratatui::buffer::Buffer, row: u16, start_col: u16, max_col: u16, exclude: Option<Rect>) -> u16 {
        let mut end = start_col;
        for column in start_col..=max_col {
            if exclude.is_some_and(|area| rect_contains(area, column, row)) {
                continue;
            }
            if let Some(cell) = buffer.cell((column, row))
                && !cell.symbol().trim().is_empty()
            {
                end = column;
            }
        }
        end
    }

    /// Paints a selection as reverse-video cells directly on the completed buffer, a
    /// post-process pass over whatever `render` already drew, rather than something any
    /// individual node's own rendering needs to know about. Each row is trimmed to its real
    /// text extent via `content_end_column`, not the full `selection.bounds` width. A plain
    /// click with no drag (`start == end`) renders nothing, rather than a single stray
    /// reverse-video cell that looks like a leftover cursor. `overlay_rect`, when given, is
    /// skipped entirely unless the selection actually started on the overlay itself (checked via
    /// `selection.anchor`), so an overlay floating over this selection's own node doesn't get
    /// painted as if it were part of the selection.
    fn render_selection_highlight(frame: &mut Frame, selection: &Selection, overlay_rect: Option<Rect>) {
        let (start, end) = selection.ordered();

        if start == end {
            return;
        }

        let exclude = overlay_rect.filter(|area| !rect_contains(*area, selection.anchor.0, selection.anchor.1));

        let bounds = selection.bounds;
        let frame_area = frame.area();
        let highlight_style = Style::new().reversed();
        let max_col = bounds.x + bounds.width.saturating_sub(1);
        let buffer = frame.buffer_mut();

        for row in start.1..=end.1 {
            let column_start = if row == start.1 { start.0 } else { bounds.x };
            let raw_column_end = if row == end.1 { end.0 } else { max_col };
            let column_end = Self::content_end_column(buffer, row, column_start, raw_column_end.max(column_start), exclude);

            if column_end < column_start {
                continue;
            }

            for column in column_start..=column_end {
                if exclude.is_some_and(|area| rect_contains(area, column, row)) {
                    continue;
                }

                let cell_rect = Rect { x: column, y: row, width: 1, height: 1 }.intersection(frame_area);

                if cell_rect.width > 0 && cell_rect.height > 0 {
                    buffer.set_style(cell_rect, highlight_style);
                }
            }
        }
    }

    /// Reads the selected cells' text back out of the just-rendered buffer, in reading order,
    /// and copies it to the system clipboard. Same real-text-extent trim as the highlight.
    /// Copying should get exactly what's visibly highlighted, not a wider span padded with
    /// blank cells. A plain click with no drag (`start == end`) copies nothing. `overlay_rect`
    /// is excluded the same way and for the same reason as in `render_selection_highlight`; a
    /// skipped overlay gap collapses to a single space rather than joining the real text on
    /// either side of it into one word.
    fn copy_selection_to_clipboard(buffer: &ratatui::buffer::Buffer, selection: &Selection, overlay_rect: Option<Rect>) {
        let (start, end) = selection.ordered();

        if start == end {
            return;
        }

        let exclude = overlay_rect.filter(|area| !rect_contains(*area, selection.anchor.0, selection.anchor.1));

        let bounds = selection.bounds;
        let max_col = bounds.x + bounds.width.saturating_sub(1);
        let mut text = String::new();

        for row in start.1..=end.1 {
            let column_start = if row == start.1 { start.0 } else { bounds.x };
            let raw_column_end = if row == end.1 { end.0 } else { max_col };
            let column_end = Self::content_end_column(buffer, row, column_start, raw_column_end.max(column_start), exclude);

            let mut skipped_a_cell = false;
            for column in column_start..=column_end.max(column_start) {
                if exclude.is_some_and(|area| rect_contains(area, column, row)) {
                    skipped_a_cell = true;
                    continue;
                }

                if skipped_a_cell {
                    text.push(' ');
                    skipped_a_cell = false;
                }

                if let Some(cell) = buffer.cell((column, row)) {
                    text.push_str(cell.symbol());
                }
            }

            if row != end.1 {
                text.push('\n');
            }
        }

        // A selection with no printable content isn't worth clobbering whatever the user
        // already had on their clipboard.
        if text.trim().is_empty() {
            return;
        }

        match arboard::Clipboard::new() {
            Ok(mut clipboard) => {
                if let Err(error) = clipboard.set_text(text) {
                    tracing::warn!("Failed to copy selection to clipboard: {error}");
                }
            }
            Err(error) => tracing::warn!("Failed to access system clipboard: {error}"),
        }
    }
}

impl<B: Backend> TerminalSurface<B> {
    pub fn clear(&mut self) -> Result<(), B::Error> {
        self.terminal.clear()
    }
}

//---
pub fn into_margin(margin: &escher_core::style::Margin) -> ratatui::widgets::Padding {
    match margin.1 {
        Value::Px(px) => match margin.0 {
            Edge::All => ratatui::widgets::Padding::uniform(px.as_()),
            Edge::Top => ratatui::widgets::Padding::top(px.as_()),
            Edge::Right => ratatui::widgets::Padding::right(px.as_()),
            Edge::Bottom => ratatui::widgets::Padding::bottom(px.as_()),
            Edge::Left => ratatui::widgets::Padding::left(px.as_()),
            Edge::None => ratatui::widgets::Padding::ZERO,
        },
        unhandled_value => {
            // TODO: Document this behavior in Known Issues.
            tracing::warn!("Margin not yet implemented for value {:?}; using default ..", unhandled_value);
            ratatui::widgets::Padding::default()
        }
    }
}

pub fn into_padding(padding: &escher_core::style::Padding) -> ratatui::widgets::Padding {
    match padding.1 {
        Value::Px(px) => match padding.0 {
            Edge::All => ratatui::widgets::Padding::uniform(px.as_()),
            Edge::Top => ratatui::widgets::Padding::top(px.as_()),
            Edge::Right => ratatui::widgets::Padding::right(px.as_()),
            Edge::Bottom => ratatui::widgets::Padding::bottom(px.as_()),
            Edge::Left => ratatui::widgets::Padding::left(px.as_()),
            Edge::None => ratatui::widgets::Padding::ZERO,
        },
        unhandled_value => {
            tracing::warn!("Padding not yet implemented for value {:?}; using default ..", unhandled_value);
            ratatui::widgets::Padding::default()
        }
    }
}

/// Clamps `point` into `rect`, used to keep a selection's drag/up coordinate inside the
/// content node it started in, even though the raw mouse position it's built from can land
/// anywhere on screen.
fn clamp_to_rect(point: (u16, u16), rect: Rect) -> (u16, u16) {
    let x = point.0.clamp(rect.x, rect.x + rect.width.saturating_sub(1));
    let y = point.1.clamp(rect.y, rect.y + rect.height.saturating_sub(1));
    (x, y)
}

/// Whether `(column, row)` falls inside `rect`.
fn rect_contains(rect: Rect, column: u16, row: u16) -> bool {
    column >= rect.x && column < rect.x + rect.width && row >= rect.y && row < rect.y + rect.height
}

/// Accumulates per-edge insets from multiple `Margin`/`Padding` entries (e.g. a separate
/// `Margin::top(..)` and `Margin::left(..)` both set on the same node) into one total.
pub fn sum_insets(insets: impl Iterator<Item = ratatui::widgets::Padding>) -> ratatui::widgets::Padding {
    insets.fold(ratatui::widgets::Padding::ZERO, |total, inset| ratatui::widgets::Padding {
        left: total.left + inset.left,
        right: total.right + inset.right,
        top: total.top + inset.top,
        bottom: total.bottom + inset.bottom,
    })
}

/// Pads every line of `content` with trailing spaces out to `width` display columns (ANSI
/// escapes don't count toward width, see `escher_core::content::display_width`), so a
/// `Paragraph`'s background fill covers the whole line instead of stopping wherever its own
/// text happens to end.
fn pad_lines_to_width(content: &str, width: usize) -> String {
    content
        .split('\n')
        .map(|line| {
            let line_width = display_width(line);
            if line_width < width {
                let mut padded = String::with_capacity(line.len() + (width - line_width));
                padded.push_str(line);
                padded.extend(std::iter::repeat(' ').take(width - line_width));
                padded
            } else {
                line.to_string()
            }
        })
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn into_direction(flex_direction: FlexDirection) -> ratatui::layout::Direction {
    match flex_direction {
        FlexDirection::Column => Direction::Vertical,
        FlexDirection::Row => Direction::Horizontal,
    }
}

pub fn into_constraint(size: &Size, direction: FlexDirection, area: &Rect) -> Constraint {
    match size {
        Size(Value::Px(width), _, _) => Constraint::Length(width.as_()),
        Size(_, Value::Px(height), _) => Constraint::Length(height.as_()),
        Size(_, _, _) => match direction {
            FlexDirection::Row => Constraint::Length(area.width),
            FlexDirection::Column => Constraint::Length(area.height),
        },
    }
}

fn unpack_keyboard_event(event: &KeyEvent) -> KeyboardEvent {
    KeyboardEvent {
        state: unpack_state(&event.kind),
        key: unpack_key(&event.code),
        code: unpack_code(&event.code),
        location: unpack_location(&event.state),
        modifiers: unpack_modifiers(&event.modifiers),
        repeat: matches!(&event.kind, KeyEventKind::Repeat),
        is_composing: false, // ??
    }
}

fn unpack_modifiers(modifiers: &KeyModifiers) -> Modifiers {
    match modifiers {
        &KeyModifiers::ALT => Modifiers::ALT,
        &KeyModifiers::SHIFT => Modifiers::SHIFT,
        &KeyModifiers::CONTROL => Modifiers::CONTROL,
        &KeyModifiers::SUPER => Modifiers::META,
        &KeyModifiers::HYPER => Modifiers::META,
        &KeyModifiers::META => Modifiers::META,
        #[allow(unused)]
        unhandled_modifier => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key Modifier: {:?}", unhandled_modifier);
            Modifiers::empty()
        }
    }
}

fn unpack_state(code: &KeyEventKind) -> KeyState {
    match code {
        KeyEventKind::Press => KeyState::Down,
        KeyEventKind::Repeat => KeyState::Down,
        KeyEventKind::Release => KeyState::Up,
    }
}

fn unpack_key(code: &KeyCode) -> Key {
    match code {
        KeyCode::Esc => Key::Named(NamedKey::Escape),
        KeyCode::Backspace => Key::Named(NamedKey::Backspace),
        #[allow(unused)]
        unhandled_key => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key: {:?}", unhandled_key);
            Key::default()
        }
    }
}

fn unpack_code(code: &KeyCode) -> Code {
    match code {
        KeyCode::Esc => Code::Escape,
        KeyCode::Backspace => Code::Backspace,
        #[allow(unused)]
        unhandled_code => {
            #[cfg(all(feature="dev", feature="verbose"))]
            tracing::warn!("Unhandled Key: {:?}", unhandled_code);
            Code::default()
        }
    }
}

fn unpack_location(state: &KeyEventState) -> Location {
    Location::Standard
}
