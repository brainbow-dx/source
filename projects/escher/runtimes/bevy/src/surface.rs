//! Renders an `escher_core::scaffold::Scaffold` tree into Bevy UI nodes.
//!
//! This is the Bevy-side counterpart to `escher_terminal::surface::TerminalSurface`, built the
//! same way: a bump-allocated `Scaffold` is constructed fresh from a caller-supplied `draw_fn`
//! and walked recursively to emit real UI output. It does *not* implement
//! `escher_core::surface::Surface` — that trait's `fn draw(&mut self, draw_fn) -> Result<Event>`
//! has no way to receive Bevy's `&mut Commands` (ECS mutation is only valid for the duration of
//! a system call, not something a struct can hold onto), so `BevySurface::draw` takes `Commands`
//! and a root `Entity` as extra arguments instead of trying to force-fit the trait.
//!
//! Unlike the pre-refactor `provider.rs` (kept in `src/legacy/` for reference), this rebuilds the
//! whole subtree under `root` on every draw call (despawn children, respawn from scratch) rather
//! than diffing add/update/remove per node — `provider.rs`'s diffing was itself mostly TODO
//! (`Update`/`Remove` were unimplemented stubs; only `Add` worked), so a full rebuild is both
//! simpler and more complete. It's not free — fine for a redraw-on-a-timer or redraw-on-input
//! scene, not a good fit yet for a UI that redraws every frame at 60fps.

use bevy::color::Color as BevyColor;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Commands;
use bevy::prelude::ChildOf;
use bevy::text::TextColor;
use bevy::text::TextFont;
use bevy::ui::widget::Text as BevyText;
use bevy::ui::BackgroundColor;
use bevy::ui::BorderColor;
use bevy::ui::FlexDirection as BevyFlexDirection;
use bevy::ui::Node;
use bevy::ui::PositionType;
use bevy::ui::UiRect;
use bevy::ui::Val;

use bumpalo_herd::Herd;

use escher_core::scaffold::Scaffold;
use escher_core::style::Border;
use escher_core::style::Color as EscherColor;
use escher_core::style::Edge;
use escher_core::style::Flex;
use escher_core::style::FlexDirection;
use escher_core::style::Gap;
use escher_core::style::OverlayInset;
use escher_core::style::Property;
use escher_core::style::Size;
use escher_core::style::Value;

/// The default text size used for all content, since `escher_core::style` doesn't (yet) have a
/// `FontSize` property to read one from — the pre-refactor core did.
const DEFAULT_FONT_SIZE: f32 = 16.0;

pub struct BevySurface {
    allocator: Herd,
}

impl BevySurface {
    pub fn new() -> Self {
        BevySurface { allocator: Herd::new() }
    }
}

impl Default for BevySurface {
    fn default() -> Self {
        BevySurface::new()
    }
}

impl BevySurface {
    /// Builds a `Scaffold` via `draw_fn`, then despawns whatever was previously spawned under
    /// `root` and respawns it fresh from the new tree.
    pub fn draw<F>(&mut self, commands: &mut Commands, root: Entity, draw_fn: F)
    where
        F: for<'ctx> FnOnce(Scaffold<'ctx>) -> Scaffold<'ctx>,
    {
        let arena = self.allocator.get();
        let scaffold = draw_fn(Scaffold::new_in(arena.as_bump()));

        commands.entity(root).despawn_children();

        if scaffold.is_enabled() {
            Self::spawn_node(commands, root, &scaffold);
        }

        if let Some(overlay) = scaffold.get_overlay()
            && overlay.is_enabled()
        {
            Self::spawn_overlay(commands, root, overlay);
        }
    }
}

impl BevySurface {
    fn spawn_node(commands: &mut Commands, parent: Entity, scaffold: &Scaffold) {
        let (node, background_color, border_color) = Self::build_node(scaffold);

        let entity_commands = commands.spawn((node, background_color, border_color, ChildOf(parent)));
        let entity = entity_commands.id();

        if let Some(content) = scaffold.get_content() {
            Self::spawn_text(commands, entity, scaffold, &content.to_string());
        }

        for (_, child) in scaffold.get_slots().iter() {
            if child.is_enabled() {
                Self::spawn_node(commands, entity, child);
            }
        }
    }

    /// Overlays skip the normal parent/child layout flow — anchored to the bottom-right corner
    /// of `parent` via `PositionType::Absolute`, the same corner `TerminalSurface::overlay_rect`
    /// anchors to, honoring the same `OverlayInset` style.
    fn spawn_overlay(commands: &mut Commands, parent: Entity, overlay: &Scaffold) {
        let (mut node, background_color, border_color) = Self::build_node(overlay);

        node.position_type = PositionType::Absolute;
        node.right = Val::Px(0.0);
        node.bottom = Val::Px(0.0);

        for property in overlay.get_styles().iter().flat_map(|(_, values)| values) {
            if let Property::OverlayInset(OverlayInset(edge, value)) = property {
                let inset = into_val(value);
                match edge {
                    Edge::Bottom => node.bottom = inset,
                    Edge::Right => node.right = inset,
                    Edge::All => {
                        node.bottom = inset;
                        node.right = inset;
                    }
                    _ => {}
                }
            }
        }

        let entity_commands = commands.spawn((node, background_color, border_color, ChildOf(parent)));
        let entity = entity_commands.id();

        if let Some(content) = overlay.get_content() {
            Self::spawn_text(commands, entity, overlay, &content.to_string());
        }

        for (_, child) in overlay.get_slots().iter() {
            if child.is_enabled() {
                Self::spawn_node(commands, entity, child);
            }
        }
    }

    fn spawn_text(commands: &mut Commands, parent: Entity, scaffold: &Scaffold, content: &str) {
        let mut text_color = TextColor::default();

        for property in scaffold.get_styles().iter().flat_map(|(_, values)| values) {
            if let Property::ContentColor(color) = property
                && let Some(color) = unpack_color(color)
            {
                text_color = TextColor(color);
            }
        }

        commands.spawn((
            BevyText::new(content.to_string()),
            text_color,
            TextFont::default().with_font_size(DEFAULT_FONT_SIZE),
            ChildOf(parent),
        ));
    }

    fn build_node(scaffold: &Scaffold) -> (Node, BackgroundColor, BorderColor) {
        let mut node = Node::default();
        let mut background_color = BackgroundColor::default();
        let mut border_color = BorderColor::DEFAULT;

        for property in scaffold.get_styles().iter().flat_map(|(_, values)| values) {
            match property {
                Property::Size(size) => apply_size(size, &mut node),
                Property::Margin(margin) => apply_edge(margin.0, into_val(&margin.1), &mut node.margin),
                Property::Padding(padding) => apply_edge(padding.0, into_val(&padding.1), &mut node.padding),
                Property::Gap(Gap(value)) => {
                    let gap = into_val(value);
                    node.row_gap = gap;
                    node.column_gap = gap;
                }
                Property::Flex(Flex(unit)) => node.flex_grow = unit.0 as f32,
                Property::FlexDirection(direction) => node.flex_direction = into_flex_direction(*direction),
                Property::BackgroundColor(color) => {
                    if let Some(color) = unpack_color(color) {
                        background_color = BackgroundColor(color);
                    }
                }
                Property::Border(Border(edge, value, _style, color)) => {
                    apply_edge(*edge, into_val(value), &mut node.border);
                    if let Some(color) = unpack_color(color) {
                        border_color = BorderColor::all(color);
                    }
                }
                // Heading/FontStyle/FontWeight/TextDecorationLine/TextAlign/Overflow/
                // ScrollPosition have no Bevy UI equivalent wired up yet — a simple scene
                // doesn't need them, and `TerminalSurface` is the reference for how each would
                // map if a future scene needs them.
                _ => {}
            }
        }

        (node, background_color, border_color)
    }
}

fn into_val(value: &Value) -> Val {
    match value {
        Value::Auto => Val::Auto,
        Value::Px(unit) => Val::Px(unit.0 as f32),
        Value::Percent(unit) => Val::Percent(unit.0 as f32),
        // No direct Bevy `Val` equivalent for a CSS-`flex-basis`-style "fill remaining space"
        // sizing value; `Percent(100)` is the closest approximation for a single-child box.
        Value::Fill(_) => Val::Percent(100.0),
    }
}

fn apply_edge(edge: Edge, value: Val, rect: &mut UiRect) {
    match edge {
        Edge::All => {
            rect.top = value;
            rect.right = value;
            rect.bottom = value;
            rect.left = value;
        }
        Edge::Top => rect.top = value,
        Edge::Right => rect.right = value,
        Edge::Bottom => rect.bottom = value,
        Edge::Left => rect.left = value,
        Edge::None => {}
    }
}

fn apply_size(size: &Size, node: &mut Node) {
    let Size(width, height, _depth) = size;
    node.width = into_val(width);
    node.height = into_val(height);
}

fn into_flex_direction(direction: FlexDirection) -> BevyFlexDirection {
    match direction {
        FlexDirection::Row => BevyFlexDirection::Row,
        FlexDirection::Column => BevyFlexDirection::Column,
    }
}

fn unpack_color(color: &EscherColor) -> Option<BevyColor> {
    color.map(|linear| BevyColor::srgba_u8(linear.red, linear.green, linear.blue, linear.alpha))
}
