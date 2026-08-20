//! Renders the play area as one dimmed backdrop with bright, per-player sprites, ghosts, and
//! effects spliced on top. The terminal surface has no free-standing 2D drawing primitive, so a
//! frame is built as plain text: every non-sprite cell renders dim, and each live element overrides
//! or tints exactly the cells it occupies.

use std::fmt::Write as _;

use bevy::ecs::entity::Entity;
use color_eyre::owo_colors::OwoColorize;
use escher_core::animate::lerp_color;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::physics::MarioState;
use crate::physics::MARIO_ATTACK_EFFECT_DURATION;
use crate::physics::MARIO_ATTACK_FLASH_MAX_BLEND;
use crate::physics::MARIO_ATTACK_TINT_MAX_BLEND;
use crate::physics::MARIO_ATTACK_VISUAL_REACH;
use crate::physics::MARIO_ATTACK_REACH;
use crate::physics::MARIO_DEATH_EFFECT_DURATION;
use crate::physics::MARIO_DUST_EFFECT_DURATION;
use crate::physics::MARIO_STARTING_LIVES;
use crate::ghosts::mario_ghost_glyph;

pub const DIM: (u8, u8, u8) = (60, 65, 90);
pub const ACCENT_BLUE: (u8, u8, u8) = (122, 162, 247);
pub const ACCENT_ORANGE: (u8, u8, u8) = (224, 175, 104);

/// Every live player, effect, and platform renders as a solid background-color block rather than
/// a colored character on the terminal's own default background. This is the user's own explicit
/// direction, after finding a previous rainbow-of-foreground-glyphs look "totally stood out"
/// against itself. `texture_fg_for` always derives a readable foreground for whatever glyph sits
/// on top of a block from that block's own color, so no effect needs to hand-pick a contrasting
/// pair of colors.
fn texture_fg_for(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let luminance = 0.2126 * bg.0 as f64 + 0.7152 * bg.1 as f64 + 0.0722 * bg.2 as f64;
    if luminance > 140.0 {
        lerp_color(bg, (0, 0, 0), 0.45)
    } else {
        lerp_color(bg, (255, 255, 255), 0.45)
    }
}

/// One glyph per player slot, on top of that player's own bright block color
/// (`physics::mario_player_color`). It is not yet player-chosen (see the module doc above this
/// file's usage sites); this is the deterministic placeholder every player index maps to until a
/// real picker exists.
const PLAYER_FLAIRS: [char; 4] = ['#', '@', '%', '&'];

pub fn mario_player_flair(player_index: usize) -> char {
    PLAYER_FLAIRS[player_index % PLAYER_FLAIRS.len()]
}

/// The single static platform's fill: a warm, neutral stone tone, deliberately not in the same
/// blue-gray family as `DIM`/the player blocks so a platform reads as solid ground underfoot
/// rather than more backdrop.
const PLATFORM_COLOR: (u8, u8, u8) = (94, 84, 74);
const PLATFORM_TEXTURE: char = '=';
/// How many extra rows of solid texture are painted *below* the platform's own top-surface row,
/// independent of the collider's real (deliberately paper-thin, ~0.02 fraction) thickness. This is
/// a real, live-verified bug, found by tracing the actual rendered frame: with no visual thickness
/// at all, a platform is exactly one character row, indistinguishable in kind from any other single
/// line of text or from the ground itself, so standing on it never read as "on top of a raised
/// block." That matches the user's own "lands in the center" and "walks the same line as the
/// floor" reports. Collision
/// still resolves against the real thin rect (`physics::spawn_platform`) unchanged; this only
/// gives the painted block a body to stand visibly above.
const PLATFORM_VISUAL_EXTRA_ROWS: u16 = 2;

/// Hard-wraps `text` to `width` display columns, breaking purely on column count with no word
/// awareness.
pub fn wrap_to_columns(text: &str, width: usize) -> Vec<String> {
    if width == 0 {
        return text.split('\n').map(str::to_string).collect();
    }

    let mut rows = Vec::new();
    for line in text.split('\n') {
        let mut current = String::new();
        let mut current_width = 0usize;
        for c in line.chars() {
            let char_width = UnicodeWidthChar::width(c).unwrap_or(0);
            if current_width > 0 && current_width + char_width > width {
                rows.push(std::mem::take(&mut current));
                current_width = 0;
            }
            current.push(c);
            current_width += char_width;
        }
        rows.push(current);
    }
    rows
}

fn pad_to_width(text: &str, width: usize) -> String {
    let text_width = UnicodeWidthStr::width(text);
    if text_width >= width {
        text.to_string()
    } else {
        format!("{text}{}", " ".repeat(width - text_width))
    }
}

/// The pause menu's short entry list. Each entry's actual effect lives in `apply_cheat`, indexed
/// by position in this array.
pub const CHEAT_ENTRIES: [&str; 2] = ["Revive all players (restore lives)", "Reset kill scores"];

/// Applies `CHEAT_ENTRIES[index]`'s effect against every locally simulated player. Silently does
/// nothing for an out-of-range index, which should never happen since the menu only ever selects
/// a real row.
pub fn apply_cheat(mario: &mut Vec<(Entity, String, MarioState)>, index: usize) {
    match index {
        0 => {
            for (_, _, mario_state) in mario.iter_mut() {
                mario_state.alive = true;
                mario_state.lives = MARIO_STARTING_LIVES;
                mario_state.respawn_remaining = None;
                mario_state.death_effect = None;
                mario_state.x = 0.5;
                mario_state.y = crate::physics::MARIO_GROUND_Y;
                mario_state.vx = 0.0;
                mario_state.vy = 0.0;
                mario_state.jumps_used = 0;
            }
        }
        1 => {
            for (_, _, mario_state) in mario.iter_mut() {
                mario_state.kills = 0;
            }
        }
        _ => {}
    }
}

/// The pause menu's literal box content: `(line, is_selected_row)` per line, plain and unstyled.
/// `mario_body_text` colors it (border one color, the selected row highlighted) and centers it
/// within the play area at render time.
pub fn cheat_menu_lines(selected: usize) -> Vec<(String, bool)> {
    const TITLE: &str = "* GAME MENU *";
    let inner_width = CHEAT_ENTRIES.iter().map(|entry| UnicodeWidthStr::width(*entry) + 4).max().unwrap_or(0).max(UnicodeWidthStr::width(TITLE) + 4);

    let mut lines = Vec::new();
    lines.push((format!("+{}+", "-".repeat(inner_width)), false));
    lines.push((format!("|{:^inner_width$}|", TITLE), false));
    lines.push((format!("+{}+", "-".repeat(inner_width)), false));
    lines.push((format!("|{:inner_width$}|", ""), false));
    for (index, entry) in CHEAT_ENTRIES.iter().enumerate() {
        let marker = if index == selected { ">" } else { " " };
        lines.push((format!("|{:<inner_width$}|", format!("  {marker} {entry}")), index == selected));
    }
    lines.push((format!("|{:inner_width$}|", ""), false));
    lines.push((format!("+{}+", "-".repeat(inner_width)), false));
    lines.push(("Up/Down select, South confirm, Start close".to_string(), false));
    lines
}

/// Everything a frame needs to render: the backdrop text plus live sprites, ghosts, and an
/// optional open menu.
pub fn mario_body_text(
    backdrop_rows: &[String],
    sprites: &[(MarioState, (u8, u8, u8), usize)],
    ghosts: &[(f32, f32, (u8, u8, u8), f64)],
    menu_lines: Option<&[(String, bool)]>,
    width: u16,
    height: u16,
    colliders: &[(f32, f32, f32, f32)],
) -> String {
    if width == 0 || height == 0 {
        return String::new();
    }

    let rows = &backdrop_rows[..backdrop_rows.len().min(height as usize)];
    let blank_rows = height as usize - rows.len();

    // Sprites grouped by row so multiple players sharing one row still render as separate splices
    // rather than clobbering each other. Each entry is (column, block color, glyph). The block
    // color is the solid background fill; its glyph's own foreground is always derived from it via
    // `texture_fg_for`, never chosen separately, so every block/glyph pair stays readable.
    let mut sprites_by_row: std::collections::HashMap<u16, Vec<(u16, (u8, u8, u8), char)>> = std::collections::HashMap::new();
    // Background-only fills (no glyph override) for a swing's own blast/motion trail. The
    // existing backdrop character shows through as the texture, recolored via `texture_fg_for`.
    let mut tints_by_row: std::collections::HashMap<u16, Vec<(u16, (u8, u8, u8))>> = std::collections::HashMap::new();
    // Foreground-only glyph overrides with no background fill at all. Ghosts render this way
    // rather than through `sprites_by_row` so they stay a plain colored dot directly on the dim
    // backdrop, never a solid block: a background memorial, not something meant to visually
    // compete with a live player for attention.
    let mut glyphs_by_row: std::collections::HashMap<u16, Vec<(u16, (u8, u8, u8), char)>> = std::collections::HashMap::new();
    let to_cell = |x: f32, y: f32| -> (u16, u16) {
        let col = (x * width.saturating_sub(1) as f32).round().clamp(0.0, width.saturating_sub(1) as f32) as u16;
        let row = (y * height.saturating_sub(1) as f32).round().clamp(0.0, height.saturating_sub(1) as f32) as u16;
        (col, row)
    };

    // The static platform(s) mario stands on: a normal, always-visible solid block spanning the
    // collider's full width, `PLATFORM_VISUAL_EXTRA_ROWS` rows deep regardless of whether anyone's
    // currently standing on it. Lowest-priority layer: sprites and tints (checked first at render
    // time) still draw on top of a platform a player is standing on or swinging over.
    let mut platforms_by_row: std::collections::HashMap<u16, (u16, u16)> = std::collections::HashMap::new();
    for &(x0, y0, x1, _y1) in colliders {
        let (col0, row0) = to_cell(x0, y0);
        let (col1, _) = to_cell(x1, y0);
        let row1 = row0.saturating_add(PLATFORM_VISUAL_EXTRA_ROWS).min(height.saturating_sub(1));
        for row in row0..=row1 {
            platforms_by_row.entry(row).and_modify(|(min_col, max_col)| {
                *min_col = (*min_col).min(col0);
                *max_col = (*max_col).max(col1);
            }).or_insert((col0, col1));
        }
    }

    for (mario, identity_color, player_index) in sprites {
        let block_color = *identity_color;
        let flair = mario_player_flair(*player_index);

        // A dead player, counting down to a respawn or permanently out, has no living block to
        // render, only its death burst below marking where it happened.
        if mario.alive {
            let (col, row) = to_cell(mario.x, mario.y);
            sprites_by_row.entry(row).or_default().push((col, block_color, flair));
        }

        if let Some(effect) = mario.dust_effect {
            let (dust_col, dust_row) = to_cell(effect.x, effect.y);
            // Fades toward the backdrop's own dim tone as `remaining` runs out, toward the
            // player's own block color rather than a fixed neutral tone, so a puff reads as that
            // player's own kick-up settling back into the background.
            let fade = (effect.remaining / MARIO_DUST_EFFECT_DURATION).clamp(0.0, 1.0) as f64;
            let dust_color = lerp_color(DIM, block_color, fade);
            sprites_by_row.entry(dust_row).or_default().push((dust_col, dust_color, 'o'));
        }

        if let Some(effect) = mario.attack_effect {
            // A short solid extension of the avatar's own body: the swing reads as the avatar
            // itself reaching out, not a separate mark appearing near it. Flashes toward white at
            // the instant of the swing, then settles to the player's own block color as it fades.
            let fade = (effect.remaining / MARIO_ATTACK_EFFECT_DURATION).clamp(0.0, 1.0) as f64;
            let swipe_color = lerp_color(block_color, (255, 255, 255), fade * MARIO_ATTACK_FLASH_MAX_BLEND);
            for offset_fraction in [0.3_f32, 0.55_f32, 0.8_f32] {
                let (swipe_col, swipe_row) =
                    to_cell(mario.x + effect.dx * MARIO_ATTACK_VISUAL_REACH * offset_fraction, mario.y + effect.dy * MARIO_ATTACK_VISUAL_REACH * offset_fraction);
                sprites_by_row.entry(swipe_row).or_default().push((swipe_col, swipe_color, '/'));
            }

            // A short motion trail between the attacker and the swipe: a solid background fill,
            // lit partway toward the player's own block color, so the swing reads as moving
            // through that space rather than a mark just appearing.
            let tint_fade = fade * MARIO_ATTACK_TINT_MAX_BLEND;
            let tint_color = lerp_color(DIM, block_color, tint_fade);
            for offset_fraction in [0.12_f32, 0.26_f32] {
                let (tint_col, tint_row) =
                    to_cell(mario.x + effect.dx * MARIO_ATTACK_VISUAL_REACH * offset_fraction, mario.y + effect.dy * MARIO_ATTACK_VISUAL_REACH * offset_fraction);
                tints_by_row.entry(tint_row).or_default().push((tint_col, tint_color));
            }
        }

        if let Some(effect) = mario.death_effect {
            // A scattered spark burst expanding outward from the death spot as `remaining` counts
            // down, flashing white-hot at the instant of death and fading to nothing. Radius grows
            // with elapsed time rather than shrinking with `remaining`, since an expanding-then-
            // vanishing burst reads as an explosion rather than a puff. Fanned around the reverse
            // of whatever swing killed this player at varied angles and reach, not a tidy
            // symmetric cross.
            let elapsed_fraction = (1.0 - effect.remaining / MARIO_DEATH_EFFECT_DURATION).clamp(0.0, 1.0);
            let fade = (effect.remaining / MARIO_DEATH_EFFECT_DURATION).clamp(0.0, 1.0) as f64;
            let death_color = lerp_color(block_color, (255, 255, 255), fade);
            let base_angle = effect.burst_dy.atan2(effect.burst_dx);
            for (angle_offset, reach_fraction) in
                [(0.0_f32, 1.0_f32), (0.5, 0.7), (-0.5, 0.85), (1.0, 0.55), (-1.0, 0.6), (1.7, 0.4), (-1.7, 0.45)]
            {
                let angle = base_angle + angle_offset;
                let radius = MARIO_ATTACK_REACH * 1.1 * reach_fraction * elapsed_fraction;
                let (spark_col, spark_row) = to_cell(effect.x + angle.cos() * radius, effect.y + angle.sin() * radius);
                sprites_by_row.entry(spark_row).or_default().push((spark_col, death_color, '+'));

                let glow_color = lerp_color(DIM, block_color, fade * MARIO_ATTACK_TINT_MAX_BLEND);
                let (glow_col, glow_row) = to_cell(effect.x + angle.cos() * radius * 0.55, effect.y + angle.sin() * radius * 0.55);
                tints_by_row.entry(glow_row).or_default().push((glow_col, glow_color));
            }
        }
    }

    // Every lost life, ever. Rendered through `glyphs_by_row`, not `sprites_by_row`, so a ghost is
    // always a faint colored dot directly on the plain backdrop, never a solid block. They're a
    // memorial, not something anyone's currently standing on.
    for &(x, y, color, flicker) in ghosts {
        let (col, row) = to_cell(x, y);
        let ghost_color = lerp_color(DIM, color, crate::ghosts::MARIO_GHOST_DIM_FRACTION * flicker);
        glyphs_by_row.entry(row).or_default().push((col, ghost_color, mario_ghost_glyph(flicker)));
    }

    let empty_sprites: Vec<(u16, (u8, u8, u8), char)> = Vec::new();
    let empty_tints: Vec<(u16, (u8, u8, u8))> = Vec::new();
    let empty_glyphs: Vec<(u16, (u8, u8, u8), char)> = Vec::new();

    // Centered vertically within the play area, a fixed number of rows regardless of what's
    // underneath. Each line is centered horizontally and colored as a whole, the selected row
    // brighter than the rest.
    let menu_row_start = menu_lines.map(|lines| (height as usize).saturating_sub(lines.len()) / 2);

    let mut out = String::new();
    for r in 0..height {
        if let (Some(lines), Some(start)) = (menu_lines, menu_row_start) {
            if (r as usize) >= start && (r as usize) < start + lines.len() {
                let (line, is_selected) = &lines[r as usize - start];
                let indent = " ".repeat((width as usize).saturating_sub(UnicodeWidthStr::width(line.as_str())) / 2);
                let centered = pad_to_width(&format!("{indent}{line}"), width as usize);
                let menu_color = if *is_selected { ACCENT_ORANGE } else { ACCENT_BLUE };
                let _ = write!(&mut out, "{}", centered.truecolor(menu_color.0, menu_color.1, menu_color.2));
                if r + 1 < height {
                    out.push('\n');
                }
                continue;
            }
        }

        let row_text: &str = if (r as usize) < blank_rows { "" } else { rows[r as usize - blank_rows].as_str() };
        let platform_cols = platforms_by_row.get(&r).copied();

        match (sprites_by_row.get(&r), tints_by_row.get(&r), glyphs_by_row.get(&r), platform_cols) {
            (None, None, None, None) => {
                let _ = write!(&mut out, "{}", pad_to_width(row_text, width as usize).truecolor(DIM.0, DIM.1, DIM.2));
            }
            (row_sprites, row_tints, row_glyphs, platform_cols) => out.push_str(&render_row_with_sprites(
                row_text,
                width,
                row_sprites.unwrap_or(&empty_sprites),
                row_tints.unwrap_or(&empty_tints),
                row_glyphs.unwrap_or(&empty_glyphs),
                platform_cols,
            )),
        }

        if r + 1 < height {
            out.push('\n');
        }
    }
    out
}

/// Splices `sprites` (column, block color, glyph) on top of `row_text`, already padded to `width`.
/// Every sprite and tinted column renders as a solid background-color block, its foreground always
/// derived from that block color via `texture_fg_for` rather than a separately-chosen fg, so a
/// player, an effect, and a platform all read as "a colored block with a readable glyph on it"
/// rather than colored text on the terminal's own background. `tints` (column, color) fills a
/// background without overriding the underlying character, so the backdrop's own text becomes the
/// texture showing through a motion trail or glow. `glyphs` (column, color, glyph) overrides the
/// character and its foreground with no background fill at all: a plain colored mark directly on
/// the dim backdrop, for anything (ghosts) that must never read as a solid block. `platform_cols`, if
/// the platform spans this row, is the lowest-priority layer: any column not otherwise claimed but
/// inside that range renders as solid platform instead of plain dim backdrop. Priority, high to
/// low: sprite > tint > glyph > platform > plain dim backdrop.
fn render_row_with_sprites(
    row_text: &str,
    width: u16,
    sprites: &[(u16, (u8, u8, u8), char)],
    tints: &[(u16, (u8, u8, u8))],
    glyphs: &[(u16, (u8, u8, u8), char)],
    platform_cols: Option<(u16, u16)>,
) -> String {
    let padded = pad_to_width(row_text, width as usize);
    let mut out = String::new();
    let mut run = String::new();
    // (foreground, background). Background is `None` for the plain, unfilled dim backdrop.
    let mut run_style: Option<((u8, u8, u8), Option<(u8, u8, u8)>)> = None;

    for (col, c) in padded.chars().enumerate() {
        let effective = if let Some(&(_, bg, glyph)) = sprites.iter().find(|&&(sprite_col, ..)| sprite_col as usize == col) {
            (glyph, texture_fg_for(bg), Some(bg))
        } else if let Some(&(_, bg)) = tints.iter().find(|&&(tint_col, _)| tint_col as usize == col) {
            (c, texture_fg_for(bg), Some(bg))
        } else if let Some(&(_, fg, glyph)) = glyphs.iter().find(|&&(glyph_col, ..)| glyph_col as usize == col) {
            (glyph, fg, None)
        } else if platform_cols.is_some_and(|(min_col, max_col)| col >= min_col as usize && col <= max_col as usize) {
            (PLATFORM_TEXTURE, texture_fg_for(PLATFORM_COLOR), Some(PLATFORM_COLOR))
        } else {
            (c, DIM, None)
        };
        let (effective_char, effective_fg, effective_bg) = effective;
        let style = (effective_fg, effective_bg);

        if run_style != Some(style) {
            if let Some((fg, bg)) = run_style {
                let text = std::mem::take(&mut run);
                let _ = match bg {
                    Some(bg) => write!(&mut out, "{}", text.truecolor(fg.0, fg.1, fg.2).on_truecolor(bg.0, bg.1, bg.2)),
                    None => write!(&mut out, "{}", text.truecolor(fg.0, fg.1, fg.2)),
                };
            }
            run_style = Some(style);
        }
        run.push(effective_char);
    }
    if let Some((fg, bg)) = run_style {
        let _ = match bg {
            Some(bg) => write!(&mut out, "{}", run.truecolor(fg.0, fg.1, fg.2).on_truecolor(bg.0, bg.1, bg.2)),
            None => write!(&mut out, "{}", run.truecolor(fg.0, fg.1, fg.2)),
        };
    }

    out
}
