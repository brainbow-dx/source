//! Renders the play area as one dimmed backdrop with bright, per-player sprites, ghosts, and
//! effects spliced on top, as an ANSI-colored `String` — no terminal I/O, so both the native
//! `crossterm` build and a `wasm-bindgen`/xterm.js build can hand the same text straight to
//! whatever actually draws it. The terminal surface has no free-standing 2D drawing primitive, so
//! a frame is built as plain text: every non-sprite cell renders dim, and each live element
//! overrides or tints exactly the cells it occupies.

use std::fmt::Write as _;

use owo_colors::OwoColorize;
use unicode_width::UnicodeWidthChar;
use unicode_width::UnicodeWidthStr;

use crate::ghosts::MARIO_GHOST_DIM_FRACTION;
use crate::ghosts::mario_ghost_glyph;
use crate::state::MARIO_ATTACK_EFFECT_DURATION;
use crate::state::MARIO_ATTACK_FLASH_MAX_BLEND;
use crate::state::MARIO_ATTACK_REACH;
use crate::state::MARIO_ATTACK_TINT_MAX_BLEND;
use crate::state::MARIO_ATTACK_VISUAL_REACH;
use crate::state::MARIO_DEATH_EFFECT_DURATION;
use crate::state::MARIO_DUST_EFFECT_DURATION;
use crate::state::MarioCollider;
use crate::state::MarioState;

pub const DIM: (u8, u8, u8) = (60, 65, 90);
pub const ACCENT_BLUE: (u8, u8, u8) = (122, 162, 247);
pub const ACCENT_ORANGE: (u8, u8, u8) = (224, 175, 104);

/// Linearly interpolates between two `(r, g, b)` colors at `t` (0.0 = `a`, 1.0 = `b`). Copied from
/// `escher-core`'s `animate::lerp_color` (same body) rather than depending on that crate, so this
/// crate stays dependency-free beyond `owo-colors`/`unicode-width` — see this crate's own `lib.rs`
/// doc comment on the deliberate-duplication tradeoff this build made under time pressure.
fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let lerp = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    (lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
}

/// Every live player, effect, and platform renders as a solid background-color block rather than a
/// colored character on the terminal's own default background. `texture_fg_for` always derives a
/// readable foreground for whatever glyph sits on top of a block from that block's own color.
fn texture_fg_for(bg: (u8, u8, u8)) -> (u8, u8, u8) {
    let luminance = 0.2126 * bg.0 as f64 + 0.7152 * bg.1 as f64 + 0.0722 * bg.2 as f64;
    if luminance > 140.0 {
        lerp_color(bg, (0, 0, 0), 0.45)
    } else {
        lerp_color(bg, (255, 255, 255), 0.45)
    }
}

/// One glyph per player slot, on top of that player's own bright block color
/// (`state::mario_player_color`). Not yet player-chosen; a deterministic placeholder every player
/// index maps to.
const PLAYER_FLAIRS: [char; 4] = ['#', '@', '%', '&'];

pub fn mario_player_flair(player_index: usize) -> char {
    PLAYER_FLAIRS[player_index % PLAYER_FLAIRS.len()]
}

/// The single static platform's fill: a slight, cool slate gray — just enough to read as its own
/// surface against the near-black backdrop, not full-on cold/icy. `STONE_LIGHT_FLECK` is what
/// individual speckles lighten toward (see `stone_texture_at`), not a color drawn on its own. Real,
/// live-reported history, several directions: the very first version leaned too far blue and read as
/// frozen stone/ice; the fix for *that* then made every speckle too close in value to its neighbors
/// ("too close together"); widening the range fixed that but let the darkest specks undercut
/// `PLATFORM_COLOR` itself ("too noisy"); narrowing just the dark end fixed *that* but still read
/// "too noisy and bright" overall — both anchors are pulled down ~24 units here (still the same cool
/// tint, same R/B ratio either way) and pulled ~12 units closer together on top of that, so the whole
/// family sits darker and the spread between its darkest and lightest cell is a bit tighter too.
const PLATFORM_COLOR: (u8, u8, u8) = (44, 46, 48);
const STONE_LIGHT_FLECK: (u8, u8, u8) = (110, 113, 117);

/// The underside's own, notably darker fill -- per direct user design intent, a solid platform is
/// now two entities (see `MarioCollider`'s own doc comment): a light, collidable slab on top, and a
/// dark, pass-through underside hanging below it, reading as the shadowed bottom face of a floating
/// stone slab rather than more of the same surface. Same cool hue/ratio as `PLATFORM_COLOR`, just
/// scaled down (roughly 55-60% as bright) rather than a different color entirely -- still reads as
/// the same stone family, just in shadow.
const UNDERSIDE_COLOR: (u8, u8, u8) = (26, 27, 28);
const UNDERSIDE_LIGHT_FLECK: (u8, u8, u8) = (62, 64, 66);

/// A handful of characters that read as rough, irregular stone when scattered across a surface.
/// Deliberately chunkier than thin punctuation (`=`/`.`/`:`/etc, an earlier version's real,
/// live-reported "feels cheap up close" mistake) — solid block/shade glyphs read as texture with
/// real visual weight instead of stray marks. Also deliberately disjoint from `PLAYER_FLAIRS`
/// (`#@%&`) so a background speckle is never mistaken for a player's own identity glyph.
// `pub(crate)`, not private: this session's own tests already went stale once by hardcoding a
// duplicate of this set (a literal `'='` check) instead of referencing it directly, silently
// checking nothing the moment this list changed. Exposed so the test module can check against the
// real set instead of repeating that mistake.
pub(crate) const STONE_TEXTURE_CHARS: [char; 5] = ['▓', '▒', '░', '■', '▪'];

/// One cell's own stone glyph, background, and *foreground* — deterministic, not real randomness:
/// the same cell must render identically every frame (no flicker as the platform sits there),
/// varying only by position, so a cheap positional hash stands in for an RNG. Mixes `col` and `row`
/// together (not just one) so the pattern actually scatters in both directions instead of repeating
/// as vertical or horizontal stripes.
///
/// Real, live-reported follow-up on the foreground specifically: an earlier version derived it via
/// `texture_fg_for`, the same strong ~45%-toward-white/black blend used for player sprites that need
/// to stay sharply readable against any background — against a mid-gray stone fill, that turns every
/// glyph into a stark, high-contrast mark, "standing out heavily" instead of blending in as texture.
/// Stone marks get a *soft* variation of the same stone family instead (a modest lighten or darken
/// of that cell's own background, never full black/white), so they read as shadow/highlight within
/// the rock rather than an accent drawn on top of it.
///
/// `dark`: picks `UNDERSIDE_COLOR`/`UNDERSIDE_LIGHT_FLECK` instead of `PLATFORM_COLOR`/
/// `STONE_LIGHT_FLECK` -- everything else (the hash, the glyph choice, the lighten/blend fractions)
/// is identical, so a passable underside reads as the same stone family in shadow, not a different
/// material.
fn stone_texture_at(col: u16, row: u16, dark: bool) -> (char, (u8, u8, u8), (u8, u8, u8)) {
    let (base, fleck) = if dark { (UNDERSIDE_COLOR, UNDERSIDE_LIGHT_FLECK) } else { (PLATFORM_COLOR, STONE_LIGHT_FLECK) };
    let hash = (col as u32).wrapping_mul(2654435761).wrapping_add((row as u32).wrapping_mul(40503)) ^ ((col as u32) << 7) ^ ((row as u32) << 3);
    let glyph = STONE_TEXTURE_CHARS[(hash as usize) % STONE_TEXTURE_CHARS.len()];
    // Only ever lightens, never darkens, so every speckle still reads as the same stone, just
    // catching more or less light -- not random noise unrelated to the base color. Range widened to
    // 0.6 (was 0.25) per direct "too close together" feedback -- real, visible cell-to-cell
    // contrast now, not just a barely-perceptible variation. Floor raised to 0.12 (was 0) per a
    // direct live follow-up once that widened range read as "too noisy" -- the ceiling (and so the
    // light end of the range) is untouched, only the darkest backgrounds were lifted a little closer
    // to their neighbors.
    let bg_lighten = 0.12 + ((hash >> 8) % 100) as f64 / 100.0 * 0.48;
    let background = lerp_color(base, fleck, bg_lighten);
    // Darken-branch blend pulled back to 0.15 (was 0.3): at the old fraction, a dark-branch fleck on
    // top of an already-dark background undercut *even* `PLATFORM_COLOR` itself, standing out as an
    // extra-dark speck against its lighter neighbors -- the actual source of the "too noisy" dark end.
    let foreground = if (hash >> 16) % 2 == 0 { lerp_color(background, (0, 0, 0), 0.15) } else { lerp_color(background, fleck, 0.7) };
    (glyph, background, foreground)
}

/// The other half of the "leave the collider's own row visually correct" fix, per direct
/// live-reported follow-up: "my head should collide with the bottom of the gray shape [...]
/// currently it passes through to the top edge and stops there." A collider's `y0` (top) and `y1`
/// (underside) are only ~0.02 apart — a real, correct collision gap, but small enough that `to_cell`
/// rounds *both* to the identical terminal row at any normal terminal height. `resolve_mario_
/// collisions` always snaps `y` to exactly `y0` (landed on top) or exactly `y1` (bonked the
/// underside) on contact, never an approximation, so which face is touched is unambiguous from `y`
/// alone — no new state needs threading through from physics for this, purely a render-time lookup.
///
/// **Real, live-reported follow-up bug this replaced**: an earlier version of this check compared
/// `y` against `y1` with a tiny fixed epsilon, catching only the exact instant of contact. Live
/// testing found that "flickers correctly for a single frame, then breaks back into the box every
/// time" — the epsilon was right for that one frame, but on every frame after, gravity has already
/// nudged `y` away from `y1` by more than the epsilon, even though a coarse terminal row spans *more*
/// y-distance than the collider's own thickness (or several frames of post-bonk movement while
/// gravity is still ramping up from `vy = 0`) can escape in that time — so `raw_row` kept rounding
/// right back to the block's own shared row for many frames after the bonk, reading as "passed
/// through." Fixed by checking the *row*, not a `y` epsilon: not resting exactly on top (`y > y0`)
/// and the naive row would land at-or-inside the block's own visual footprint — covers every frame
/// from the initial bonk through however long gravity takes to actually carry the player's rendered
/// row past the block, not just the first one.
///
/// `colliders` here is the *solid-only* subset a caller has already filtered to (see
/// `mario_body_text`'s own `solid_colliders`) — a pass-through underside can never be bonked, so it
/// has nothing to correct for here.
///
/// Pushes a bonking sprite clear of *both* the collision row and the slab's own visible light band
/// (`SLAB_LIGHT_BAND_ROWS`, matching `mario_body_text`'s own definition) — not just the collision
/// row alone. A bonk landing the sprite's naive row inside the light band would share a row with
/// real platform material, the exact thing the blank-row rule exists to prevent (see
/// `MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION`'s own doc comment); landing inside the *dark* underside
/// band beyond that is fine and left uncorrected, since that space is genuinely passable -- the
/// player can really be there, falling through open (if shadowed) air, not embedded in anything.
const SLAB_LIGHT_BAND_ROWS: u16 = 1;

fn sprite_render_row(x: f32, y: f32, colliders: &[(f32, f32, f32, f32)], raw_row: u16, height: u16, to_cell: &impl Fn(f32, f32) -> (u16, u16)) -> u16 {
    const TOUCH_EPSILON: f32 = 0.0001;
    for &(x0, y0, x1, _y1) in colliders {
        if x < x0 || x > x1 || y <= y0 + TOUCH_EPSILON {
            continue; // resting exactly on top, or above/unrelated to this collider -- raw_row is already correct
        }
        let (_, row0) = to_cell(x0, y0);
        let clear_of_slab = row0.saturating_add(1).saturating_add(SLAB_LIGHT_BAND_ROWS);
        if raw_row < clear_of_slab {
            return clear_of_slab.min(height.saturating_sub(1));
        }
    }
    raw_row
}

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

/// Everything a frame needs to render: the backdrop text plus live sprites, ghosts, and an optional
/// open menu (`menu_lines`, `(line, is_selected_row)` — the native example's `render::
/// cheat_menu_lines` builds this; `mario-wasm` doesn't have a pause menu yet, so it always passes
/// `None`).
pub fn mario_body_text(
    backdrop_rows: &[String],
    sprites: &[(MarioState, (u8, u8, u8), usize)],
    ghosts: &[(f32, f32, (u8, u8, u8), f64)],
    menu_lines: Option<&[(String, bool)]>,
    width: u16,
    height: u16,
    colliders: &[MarioCollider],
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
    // Background-only fills (no glyph override) for a swing's own blast/motion trail. The existing
    // backdrop character shows through as the texture, recolored via `texture_fg_for`.
    let mut tints_by_row: std::collections::HashMap<u16, Vec<(u16, (u8, u8, u8))>> = std::collections::HashMap::new();
    // Foreground-only glyph overrides with no background fill at all. Ghosts render this way rather
    // than through `sprites_by_row` so they stay a plain colored dot directly on the dim backdrop,
    // never a solid block.
    let mut glyphs_by_row: std::collections::HashMap<u16, Vec<(u16, (u8, u8, u8), char)>> = std::collections::HashMap::new();
    let to_cell = |x: f32, y: f32| -> (u16, u16) {
        let col = (x * width.saturating_sub(1) as f32).round().clamp(0.0, width.saturating_sub(1) as f32) as u16;
        let row = (y * height.saturating_sub(1) as f32).round().clamp(0.0, height.saturating_sub(1) as f32) as u16;
        (col, row)
    };

    // The static platform(s) mario stands on. `dark` (from `MarioCollider::passable`) picks light
    // stone for a solid slab's own visible body or dark stone for a pass-through underside -- see
    // `stone_texture_at`. Lowest-priority layer: sprites and tints (checked first at render time)
    // still draw on top of a platform a player is standing on or swinging over.
    //
    // A solid collider's own row (`y0`, in cell terms -- exactly where a resting player's feet
    // land) is deliberately left out of what gets painted here; its visible body starts one row
    // below that instead. This is not a color choice, it's the same principle the original "player
    // renders embedded in platform" fix established: from the player's own physical point of view,
    // gravity pulling everything down and viewed from the side, *any* row their own glyph shares
    // with platform material -- light or dark, doesn't matter -- reads as floating inside solid
    // rock. Compositing priority (sprite checked before platform, below) always shows the player's
    // own glyph correctly in their own exact cell regardless, but neighboring cells on that same row
    // would still show platform texture, and that's what actually reads wrong. A passable underside
    // has no such concern -- nothing ever rests on it.
    //
    // Two passes, solid first: a passable underside's row is positioned *relative to the solid
    // collider's own light band* (`solid_light_end + 1`), not from its own rect's fractional
    // position at all. Real, live-reported bug this fixed, screenshotted directly: a visible gap
    // between the light and dark bands, with the help caption caught inside it. Root cause -- the
    // previous version derived the underside's row from its own rect, positioned a fixed fractional
    // gap (`MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION`) below the slab's `y1`, sized to read as "about
    // one row" at the ~40-row reference height this session's own tests and browser checks always
    // ran at. The real native terminal window in the screenshot is considerably taller than that,
    // so the identical fractional gap mapped to *several* real rows instead of one -- the same class
    // of row-rounding fragility this session has now hit three separate times with fractional-
    // position math, fixed the same way each time: stop deriving a row from a fraction tuned for
    // one specific size, derive it directly from another row instead.
    //
    // Only physics filters colliders down to solid-only (see `is_grounded`/`resolve_mario_
    // collisions`'s own callers) -- render paints every collider it's given, solid or passable,
    // since a passable underside is just as real a visual surface as the slab above it, only not a
    // collidable one.
    let mut platforms_by_row: std::collections::HashMap<u16, (u16, u16, bool)> = std::collections::HashMap::new();
    let mut solid_light_end: Option<u16> = None;
    for collider in colliders.iter().filter(|c| !c.passable) {
        let (x0, y0, x1, _) = collider.rect;
        let (col0, row0) = to_cell(x0, y0);
        let (col1, _) = to_cell(x1, y0);
        let light_row = row0.saturating_add(1);
        solid_light_end = Some(solid_light_end.map_or(light_row, |current| current.max(light_row)));
        for row in light_row..=light_row.saturating_add(SLAB_LIGHT_BAND_ROWS - 1).min(height.saturating_sub(1)) {
            platforms_by_row.entry(row).and_modify(|(min_col, max_col, _)| {
                *min_col = (*min_col).min(col0);
                *max_col = (*max_col).max(col1);
            }).or_insert((col0, col1, false));
        }
    }
    for collider in colliders.iter().filter(|c| c.passable) {
        let (x0, y0, x1, _) = collider.rect;
        let (col0, _) = to_cell(x0, y0);
        let (col1, _) = to_cell(x1, y0);
        // No solid sibling this frame (shouldn't happen for the one real platform, but a passable
        // collider with nothing to hang below shouldn't panic or vanish either) -- falls back to
        // its own rect's midpoint, the old per-entity behavior.
        let row = solid_light_end.map(|end| end.saturating_add(1)).unwrap_or_else(|| to_cell(x0, (y0 + collider.rect.3) / 2.0).1);
        for row in row..=row.min(height.saturating_sub(1)) {
            platforms_by_row.entry(row).and_modify(|(min_col, max_col, _)| {
                *min_col = (*min_col).min(col0);
                *max_col = (*max_col).max(col1);
            }).or_insert((col0, col1, collider.passable));
        }
    }
    // Physics never sees `passable`, only ever the plain rects `sprite_render_row`'s own resting-
    // position correction needs -- a pass-through underside can never be bonked or rested on, so it
    // has nothing to correct for there.
    let solid_colliders: Vec<(f32, f32, f32, f32)> = colliders.iter().filter(|c| !c.passable).map(|c| c.rect).collect();

    for (mario, identity_color, player_index) in sprites {
        let block_color = *identity_color;
        let flair = mario_player_flair(*player_index);

        // A dead player, counting down to a respawn or permanently out, has no living block to
        // render, only its death burst below marking where it happened.
        if mario.alive {
            let (col, row) = to_cell(mario.x, mario.y);
            let row = sprite_render_row(mario.x, mario.y, &solid_colliders, row, height, &to_cell);
            sprites_by_row.entry(row).or_default().push((col, block_color, flair));
        }

        if let Some(effect) = mario.dust_effect {
            let (dust_col, dust_row) = to_cell(effect.x, effect.y);
            // Same row-correction the player's own body glyph gets above -- real, live-reported
            // bug this fixed: a double jump's dust puff is spawned at `self.x, self.y` (see
            // `MarioState::step`'s own double-jump branch) at the exact moment it fires, which can
            // land in the same "ambiguous row" a nearby collider forces the player's own body out
            // of. Without this, the dust could render at its raw (uncorrected) row while the body
            // glyph is pinned elsewhere, reading as "the indicator is ahead of where the avatar
            // actually is" even though both come from the same underlying position.
            let dust_row = sprite_render_row(effect.x, effect.y, &solid_colliders, dust_row, height, &to_cell);
            let fade = (effect.remaining / MARIO_DUST_EFFECT_DURATION).clamp(0.0, 1.0) as f64;
            let dust_color = lerp_color(DIM, block_color, fade);
            sprites_by_row.entry(dust_row).or_default().push((dust_col, dust_color, 'o'));
        }

        if let Some(effect) = mario.attack_effect {
            // Deliberately *not* run through `sprite_render_row` -- real, live-reported regression
            // this reverted: unlike the player's own body/dust (both real "resting exactly at y0 or
            // y1" positions collision resolution produces), a swing's swipe/tint marks are free-
            // floating cosmetic offsets with no collision applied to them at all, aimed *toward* a
            // collider on purpose (attacking up into a platform from below is a normal, intentional
            // swing). `sprite_render_row`'s "not resting on top, and inside the block's footprint,
            // so push below" rule assumes the only two valid states are top/underside rest -- for an
            // aimed swing it instead teleports the mark to the opposite side of whatever it's
            // swinging at, reading as "the attack shows on the other side of what I hit." A swing
            // visually overlapping a block's own texture slightly is a minor, expected cosmetic
            // overlap; rendering on the wrong side of it entirely is not.
            let fade = (effect.remaining / MARIO_ATTACK_EFFECT_DURATION).clamp(0.0, 1.0) as f64;
            let swipe_color = lerp_color(block_color, (255, 255, 255), fade * MARIO_ATTACK_FLASH_MAX_BLEND);
            for offset_fraction in [0.3_f32, 0.55_f32, 0.8_f32] {
                let (swipe_col, swipe_row) =
                    to_cell(mario.x + effect.dx * MARIO_ATTACK_VISUAL_REACH * offset_fraction, mario.y + effect.dy * MARIO_ATTACK_VISUAL_REACH * offset_fraction);
                sprites_by_row.entry(swipe_row).or_default().push((swipe_col, swipe_color, '/'));
            }

            let tint_fade = fade * MARIO_ATTACK_TINT_MAX_BLEND;
            let tint_color = lerp_color(DIM, block_color, tint_fade);
            for offset_fraction in [0.12_f32, 0.26_f32] {
                let (tint_col, tint_row) =
                    to_cell(mario.x + effect.dx * MARIO_ATTACK_VISUAL_REACH * offset_fraction, mario.y + effect.dy * MARIO_ATTACK_VISUAL_REACH * offset_fraction);
                tints_by_row.entry(tint_row).or_default().push((tint_col, tint_color));
            }
        }

        if let Some(effect) = mario.death_effect {
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
    // always a faint colored dot directly on the plain backdrop, never a solid block.
    for &(x, y, color, flicker) in ghosts {
        let (col, row) = to_cell(x, y);
        let ghost_color = lerp_color(DIM, color, MARIO_GHOST_DIM_FRACTION * flicker);
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
                r,
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
/// Priority, high to low: sprite > tint > glyph > platform > plain dim backdrop.
fn render_row_with_sprites(
    row_text: &str,
    width: u16,
    row: u16,
    sprites: &[(u16, (u8, u8, u8), char)],
    tints: &[(u16, (u8, u8, u8))],
    glyphs: &[(u16, (u8, u8, u8), char)],
    // `bool` is `passable` (from `MarioCollider`) -- dark stone (underside) vs light (slab).
    platform_cols: Option<(u16, u16, bool)>,
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
        } else if let Some((glyph, background, foreground)) = platform_cols.and_then(|(min_col, max_col, dark)| {
            (col >= min_col as usize && col <= max_col as usize).then(|| stone_texture_at(col as u16, row, dark))
        }) {
            (glyph, foreground, Some(background))
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
