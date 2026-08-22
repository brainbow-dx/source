//! Pure player movement/combat state: gravity, jumps (double jump, wall kick), an attack with a
//! cooldown and mild anti-spam penalty, lives and respawn. No Bevy types, no I/O, no gamepad
//! reading — see this crate's own `lib.rs` doc comment for how this relates to the native
//! example's own (currently still separately-defined) copy in `physics.rs`.

/// One player's live physics and combat state. `x`/`y` are fractions of the play area's own
/// width/height (0.0-1.0), not raw cell coordinates, so `step` never needs to know the terminal's
/// current size and a player stays proportionally in the same spot across a resize.
#[derive(Clone, Copy)]
pub struct MarioState {
    pub x: f32,
    pub y: f32,
    /// `x`/`y` as of the start of this tick's `step`, before gravity/input moved them. This is what
    /// `resolve_mario_collisions` sweeps from to catch a fast fall passing clean through a thin
    /// platform within a single tick, rather than only checking where `step` left it.
    pub prev_x: f32,
    pub prev_y: f32,
    pub vx: f32,
    pub vy: f32,
    /// Whether a jump is currently allowed. Recomputed once per tick after collision resolution,
    /// from whichever surface (floor or platform) `y` actually rests on right now, rather than
    /// re-derived from `y >= MARIO_GROUND_Y` alone, which would miss standing on a platform above
    /// the floor.
    pub grounded: bool,
    /// How many jumps have been used since the last time `grounded` became true: 0 on the ground,
    /// up to `MARIO_MAX_JUMPS` once the double jump is also spent. Reset to 0 on landing.
    pub jumps_used: u8,
    /// Which way a wall kick should push off if a wall is currently touched: `Some(1.0)` for a wall
    /// on the left (kick pushes right), `Some(-1.0)` for a wall on the right, `None` otherwise.
    /// Recomputed fresh every tick, the same way `grounded` is, so a kick only fires on live
    /// contact rather than one remembered from several ticks ago.
    pub touching_wall: Option<f32>,
    /// A brief dust puff left at the spot a double jump was triggered, so the extra jump reads as
    /// intentional rather than stick drift.
    pub dust_effect: Option<MarioDustEffect>,
    /// Which way this player currently faces: `1.0` right, `-1.0` left. Updated only when real
    /// horizontal input is present, so standing still keeps the last facing, which is what an
    /// attack aims with if the stick is neutral when the attack button is pressed.
    pub facing: f32,
    /// Counts down to 0 after an attack fires. A fresh attack can't trigger again until it reaches
    /// 0, so holding the button doesn't spam a hit every tick.
    pub attack_cooldown: f32,
    /// Counts up every tick, reset to 0 the instant an attack fires. Checked the next time an
    /// attack fires to decide whether it's "too soon" (within `MARIO_ATTACK_SPAM_WINDOW`) and
    /// should bump `attack_spam_stacks`, or far enough apart to reset it.
    pub time_since_last_attack: f32,
    /// A capped penalty for attacking rapidly back to back: each attack within `MARIO_ATTACK_
    /// SPAM_WINDOW` of the last one adds `MARIO_ATTACK_SPAM_COOLDOWN_STEP` of extra cooldown, up
    /// to `MARIO_ATTACK_SPAM_MAX_STACKS`. Resets to 0 once an attack lands outside that window.
    pub attack_spam_stacks: u8,
    pub kills: u32,
    pub attack_effect: Option<MarioAttackEffect>,
    /// Whether this player is currently in play. `false` while dead, either counting down to a
    /// respawn (see `respawn_remaining`) or permanently out once `lives` reaches 0. A dead player
    /// controls nothing and can't be hit again.
    pub alive: bool,
    /// Lives remaining, decremented on death. Every lost life still adds a permanent ghost
    /// regardless of whether it was the one that exhausted this player's lives.
    pub lives: u8,
    /// Counts down from `MARIO_RESPAWN_SECONDS` while dead with lives remaining. Once it reaches
    /// 0, this player respawns. Stays `None` forever once `lives` hits 0: that death is permanent.
    pub respawn_remaining: Option<f32>,
    pub death_effect: Option<MarioDeathEffect>,
}

/// A retro-style spark burst expanding outward from a death, fanned around the reverse of whatever
/// swing killed this player so which direction the kill came from is legible in the burst itself.
#[derive(Clone, Copy)]
pub struct MarioDeathEffect {
    pub x: f32,
    pub y: f32,
    pub remaining: f32,
    pub burst_dx: f32,
    pub burst_dy: f32,
}

/// One in-flight attack swing. `dx`/`dy` are a unit direction chosen from the left stick at the
/// moment the button was pressed, falling back to `facing` if the stick was neutral, so a swing
/// tracks wherever the swinging player currently is rather than fading in a fixed spot.
/// `has_hit`: a swing only ever lands one hit no matter how many ticks its visual stays on screen.
#[derive(Clone, Copy)]
pub struct MarioAttackEffect {
    pub dx: f32,
    pub dy: f32,
    pub remaining: f32,
    pub has_hit: bool,
    /// Whether a heavy-hit input was held the instant this swing fired. Set once at fire time, not
    /// read live again when the hit actually lands (which can be a tick or more later, by which
    /// point the input may already be released) — a heavy swing should sound heavy because of how
    /// it was thrown, not whether the input still happens to be down on impact.
    pub heavy: bool,
}

/// A brief dust puff, see `MarioState::dust_effect`. `remaining` counts down from
/// `MARIO_DUST_EFFECT_DURATION`.
#[derive(Clone, Copy)]
pub struct MarioDustEffect {
    pub x: f32,
    pub y: f32,
    pub remaining: f32,
}

/// One bright color per player, cycling past 4 gamepads, in the classic SMB2/"mario2" 4-character
/// roster order: Mario (red), Luigi (green), Peach (pink), Toad (white with a red accent; an
/// off-white here rather than pure white so it still reads as a distinct color, not "no color").
pub fn mario_player_color(player_index: usize) -> (u8, u8, u8) {
    const MARIO_RED: (u8, u8, u8) = (205, 35, 42);
    const LUIGI_GREEN: (u8, u8, u8) = (0, 166, 81);
    const PEACH_PINK: (u8, u8, u8) = (255, 105, 180);
    const TOAD_WHITE: (u8, u8, u8) = (240, 224, 214);
    match player_index % 4 {
        0 => MARIO_RED,
        1 => LUIGI_GREEN,
        2 => PEACH_PINK,
        _ => TOAD_WHITE,
    }
}

impl Default for MarioState {
    fn default() -> Self {
        MarioState {
            x: 0.5,
            y: MARIO_GROUND_Y,
            prev_x: 0.5,
            prev_y: MARIO_GROUND_Y,
            vx: 0.0,
            vy: 0.0,
            grounded: true,
            jumps_used: 0,
            touching_wall: None,
            dust_effect: None,
            facing: 1.0,
            attack_cooldown: 0.0,
            time_since_last_attack: MARIO_ATTACK_SPAM_WINDOW + 1.0,
            attack_spam_stacks: 0,
            kills: 0,
            attack_effect: None,
            alive: true,
            lives: MARIO_STARTING_LIVES,
            respawn_remaining: None,
            death_effect: None,
        }
    }
}

/// Gravity and jump velocity are expressed in real terminal rows, not `y`'s own 0.0-1.0
/// fraction-of-height units, and converted to a fractional per-tick delta using the play area's
/// current height. Without this, the same fractional velocity covers more real rows on a taller
/// terminal, so a jump visibly gets higher purely because the window did. Reference values below
/// are tuned by feel against a roughly 40-row play area.
pub const MARIO_GRAVITY_ROWS_PER_SEC2: f32 = 96.0;
pub const MARIO_JUMP_VELOCITY_ROWS_PER_SEC: f32 = -54.0;
/// Pressure-sensitive jumping: not by measuring press duration up front, but by applying much
/// stronger gravity the instant the button is released while still ascending. Holding through the
/// whole ascent never triggers this, so a held jump reaches full height, unaffected by this
/// constant at all -- only an early release ever takes this path.
///
/// 2.5 → 8.0 per direct live feedback: "the normal jump height from a short tap... is too high...
/// I want the tap to just barely bump a player, for precise movement." The initial jump velocity
/// (`MARIO_JUMP_VELOCITY_ROWS_PER_SEC`) is the same for a tap and a hold -- only this multiplier
/// tells them apart -- so a tap's apex height is `MARIO_JUMP_VELOCITY_ROWS_PER_SEC.powi(2) / (2.0 *
/// MARIO_GRAVITY_ROWS_PER_SEC2 * MARIO_JUMP_CUT_GRAVITY_MULTIPLIER)`: roughly 40% of a full jump's
/// height at the old 2.5, down to roughly 12% at 8.0 -- a real, deliberate "barely leaves the
/// ground" tap, not a redesign of the mechanism itself.
pub const MARIO_JUMP_CUT_GRAVITY_MULTIPLIER: f32 = 8.0;
/// One extra jump while airborne, noticeably smaller than the primary jump.
pub const MARIO_DOUBLE_JUMP_VELOCITY_ROWS_PER_SEC: f32 = -38.0;
pub const MARIO_MAX_JUMPS: u8 = 2;
/// How fast a wall kick snaps `vx` toward the chosen direction, faster than ordinary movement so it
/// reads as a decisive push-off rather than a normal jump that happens to also move sideways.
pub const MARIO_WALL_KICK_SPEED_MULTIPLIER: f32 = 1.5;
/// A quick omnidirectional burst on either bumper, aimed by the left stick (or `facing` if the
/// stick is neutral -- the same fallback the attack's own aim uses, see `stick_unit_direction`).
/// Applied as velocity *added* on top of whatever `vx`/`vy` already are, not a snap-to a fixed
/// speed, so the same button either amplifies existing momentum (stick aimed with it) or cuts
/// against it (stick aimed opposite) -- one primitive for both a directional boost and a counter to
/// an existing force, per direct user design intent.
///
/// Deliberately asymmetric per direct live feedback after trying a single uniform speed: sideways
/// and downward dashes read as too strong, while an upward one didn't read as enough real lift. A
/// light nudge horizontally/downward (`MARIO_DASH_SPEED_*`, well under move speed), a real vertical
/// kick specifically when aimed upward (`MARIO_DASH_LIFT_ROWS_PER_SEC`, comparable to the primary
/// jump's own `MARIO_JUMP_VELOCITY_ROWS_PER_SEC` magnitude) -- see `step`'s own dash block for which
/// applies when.
pub const MARIO_DASH_SPEED_COLUMNS_PER_SEC: f32 = 42.0;
pub const MARIO_DASH_SPEED_ROWS_PER_SEC: f32 = 22.0;
/// Bumped up slightly (62→72) per direct live feedback once the sideways/downward split landed:
/// "slightly stronger on the upward curve... a slight but useful boost if they happen to be caught
/// at the very edge of something" -- an edge-recovery nudge, not a second jump.
pub const MARIO_DASH_LIFT_ROWS_PER_SEC: f32 = 72.0;
/// How much of `MARIO_MOVE_SPEED_COLUMNS_PER_SEC` applies per second while airborne. `step` blends
/// `vx` toward the requested speed at this rate rather than snapping to it, so a mid-air direction
/// change nudges the trajectory instead of fully redirecting it.
pub const MARIO_AIR_CONTROL_COLUMNS_PER_SEC2: f32 = 160.0;
/// Horizontal speed in real terminal columns per second, converted the same way gravity is, and
/// for the same reason: a flat fractional speed would move faster on a wider terminal.
pub const MARIO_MOVE_SPEED_COLUMNS_PER_SEC: f32 = 72.0;
/// Below this, stick input is treated as neutral rather than real intent, since a worn analog
/// stick can report a small nonzero value at rest.
pub const MARIO_STICK_DEADZONE: f32 = 0.2;
/// How much of `MARIO_MOVE_SPEED_COLUMNS_PER_SEC` still applies while grounded and crouching.
pub const MARIO_CROUCH_SPEED_MULTIPLIER: f32 = 0.45;
pub const MARIO_DUST_EFFECT_DURATION: f32 = 0.35;
pub const MARIO_ATTACK_EFFECT_DURATION: f32 = 0.18;
/// Minimum time between two attacks from the same player.
pub const MARIO_ATTACK_COOLDOWN: f32 = 0.35;
pub const MARIO_ATTACK_SPAM_WINDOW: f32 = 1.2;
pub const MARIO_ATTACK_SPAM_COOLDOWN_STEP: f32 = 0.15;
/// Caps the penalty at 4 stacks (0.6s) on top of the base cooldown: a mild, capped tax, not an
/// escalating lockout.
pub const MARIO_ATTACK_SPAM_MAX_STACKS: u8 = 4;
/// How far a swing reaches from the attacker's own position, in the same 0.0-1.0 fractional space
/// `MarioState::x`/`y` live in.
pub const MARIO_ATTACK_REACH: f32 = 0.065;
/// How far the swipe's visual burst sits from the attacker, deliberately closer than the real
/// hit-detection distance so the burst reads as landing right on the attacker rather than out at
/// the hitbox's own edge.
pub const MARIO_ATTACK_VISUAL_REACH: f32 = 0.025;
/// Ceiling on how far a swing's or death burst's background tint ever blends toward the player's
/// own color: a soft trail, never bright enough to compete with the sprite glyphs on top of it.
pub const MARIO_ATTACK_TINT_MAX_BLEND: f64 = 0.4;
/// Ceiling on how far the swipe glyph's own flash-to-white blend goes: short of pure white.
pub const MARIO_ATTACK_FLASH_MAX_BLEND: f64 = 0.55;
/// The extra downward dive velocity a stomp snaps to the instant it triggers.
pub const MARIO_STOMP_DIVE_VELOCITY_ROWS_PER_SEC: f32 = 70.0;
pub const MARIO_STARTING_LIVES: u8 = 3;
/// How long a death with lives remaining waits before respawning.
pub const MARIO_RESPAWN_SECONDS: f32 = 5.0;
pub const MARIO_DEATH_EFFECT_DURATION: f32 = 0.6;
/// `y` at rest: the play area's very last row, immediately above the footer.
pub const MARIO_GROUND_Y: f32 = 1.0;
/// Vertical tolerance for "resting exactly on a platform's top edge". Comparing floats for exact
/// equality is fragile, so this allows the tiny drift a collision or gravity step can introduce
/// before the next grounded check runs.
const MARIO_GROUNDED_EPSILON: f32 = 0.002;

/// A unit vector toward wherever the left stick is pointing, or straight along `facing` (no
/// vertical component) if the stick is centered -- shared by the attack's own aim and the dash
/// burst, both of which need "aim where I'm pointing, or aim where I'm facing if I'm not pointing
/// anywhere."
fn stick_unit_direction(move_input: f32, stick_y: f32, facing: f32) -> (f32, f32) {
    if move_input.abs() > MARIO_STICK_DEADZONE || stick_y.abs() > MARIO_STICK_DEADZONE {
        let length = (move_input * move_input + stick_y * stick_y).sqrt().max(f32::EPSILON);
        (move_input / length, stick_y / length)
    } else {
        (facing, 0.0)
    }
}

impl MarioState {
    /// Advances the simulation by `dt` seconds. Pure: no Bevy types, no I/O, so the gamepad-reading
    /// (or, in `mario-wasm`'s case, keyboard-reading) system stays a thin adapter over this and this
    /// itself stays independently testable. `move_input`: -1.0 (full left) to 1.0 (full right).
    /// `jump_held`: still down this tick, separate from the press edge so a held-vs-tapped button
    /// can be told apart at all, which is what pressure-sensitive jumping needs. `body_height`/
    /// `body_width`: the play area's current height/width in rows/columns. `crouch_held`: only
    /// actually crouches while `grounded`, pressing down in the air still just aims a downward
    /// stomp. `dash_pressed`: edge-triggered (like `jump_pressed`/`attack_pressed`), a bumper burst
    /// in the left stick's current direction -- see `MARIO_DASH_SPEED_COLUMNS_PER_SEC`.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt: f32,
        move_input: f32,
        stick_y: f32,
        jump_pressed: bool,
        jump_held: bool,
        attack_pressed: bool,
        heavy_held: bool,
        crouch_held: bool,
        dash_pressed: bool,
        body_height: f32,
        body_width: f32,
    ) {
        self.prev_x = self.x;
        self.prev_y = self.y;

        let crouching = self.grounded && crouch_held;
        let speed_multiplier = if crouching { MARIO_CROUCH_SPEED_MULTIPLIER } else { 1.0 };
        let target_vx = (move_input.clamp(-1.0, 1.0) * MARIO_MOVE_SPEED_COLUMNS_PER_SEC * speed_multiplier) / body_width;
        if self.grounded {
            self.vx = target_vx;
        } else {
            let max_delta = (MARIO_AIR_CONTROL_COLUMNS_PER_SEC2 / body_width) * dt;
            self.vx += (target_vx - self.vx).clamp(-max_delta, max_delta);
        }
        if move_input.abs() > MARIO_STICK_DEADZONE {
            self.facing = move_input.signum();
        }

        if self.grounded && jump_pressed {
            self.vy = MARIO_JUMP_VELOCITY_ROWS_PER_SEC / body_height;
            self.jumps_used = 1;
        } else if !self.grounded && jump_pressed && self.jumps_used < MARIO_MAX_JUMPS {
            self.vy = MARIO_DOUBLE_JUMP_VELOCITY_ROWS_PER_SEC / body_height;
            self.jumps_used += 1;
            self.dust_effect = Some(MarioDustEffect { x: self.x, y: self.y, remaining: MARIO_DUST_EFFECT_DURATION });

            // A wall kick spends this same double-jump charge rather than a third jump resource,
            // but snaps `vx` toward the left stick's direction (or straight away from the wall if
            // the stick is neutral) instead of the usual air-control blend, so it reads as a
            // deliberate push-off.
            if let Some(wall_push) = self.touching_wall {
                let kick_direction = if move_input.abs() > MARIO_STICK_DEADZONE { move_input.signum() } else { wall_push };
                self.vx = (kick_direction * MARIO_MOVE_SPEED_COLUMNS_PER_SEC * MARIO_WALL_KICK_SPEED_MULTIPLIER) / body_width;
            }
        }

        if dash_pressed {
            let (dx, dy) = stick_unit_direction(move_input, stick_y, self.facing);
            self.vx += (dx * MARIO_DASH_SPEED_COLUMNS_PER_SEC) / body_width;
            // `dy < 0.0` is upward (see `MarioState::y`'s down-is-positive convention) -- a real
            // lift there, a light nudge everywhere else (downward or level).
            //
            // The lift scales with `dy` *squared*, not linear, per direct live feedback: "it should
            // follow an arc [...] if the player is pointing the joystick up, the arc is tighter,
            // meaning the bump is slightly higher but takes less distance. The closer it is to any
            // other direction, the more even it is." A linear scale gave even a 45-degree diagonal
            // dash almost the same strong lift as a purely vertical one (`sin(45°) ≈ 0.71`, i.e.
            // 71% of full lift) -- the actual "too high in some conditions" bug. Squaring leaves a
            // genuinely vertical aim's lift untouched (`1.0² = 1.0`) while a 45-degree dash drops to
            // half (`0.71² ≈ 0.5`), and anything closer to level barely lifts at all -- only a real,
            // close-to-straight-up aim gets the tight, high, short-distance arc; everything else
            // reads as the flatter, more horizontally-even burst described above.
            let vertical_magnitude = if dy < 0.0 { MARIO_DASH_LIFT_ROWS_PER_SEC } else { MARIO_DASH_SPEED_ROWS_PER_SEC };
            self.vy += dy.signum() * dy.abs().powi(2) * vertical_magnitude / body_height;
        }

        let gravity = if self.vy < 0.0 && !jump_held { MARIO_GRAVITY_ROWS_PER_SEC2 * MARIO_JUMP_CUT_GRAVITY_MULTIPLIER } else { MARIO_GRAVITY_ROWS_PER_SEC2 };
        self.vy += (gravity / body_height) * dt;
        self.x = (self.x + self.vx * dt).clamp(0.0, 1.0);
        self.y += self.vy * dt;

        if self.y >= MARIO_GROUND_Y {
            self.y = MARIO_GROUND_Y;
            self.vy = 0.0;
        }

        if let Some(effect) = &mut self.dust_effect {
            effect.remaining -= dt;
            if effect.remaining <= 0.0 {
                self.dust_effect = None;
            }
        }

        self.attack_cooldown = (self.attack_cooldown - dt).max(0.0);
        self.time_since_last_attack += dt;
        if attack_pressed && self.attack_cooldown <= 0.0 {
            self.attack_spam_stacks =
                if self.time_since_last_attack < MARIO_ATTACK_SPAM_WINDOW { (self.attack_spam_stacks + 1).min(MARIO_ATTACK_SPAM_MAX_STACKS) } else { 0 };
            self.time_since_last_attack = 0.0;
            self.attack_cooldown = MARIO_ATTACK_COOLDOWN + self.attack_spam_stacks as f32 * MARIO_ATTACK_SPAM_COOLDOWN_STEP;

            let (dx, dy) = stick_unit_direction(move_input, stick_y, self.facing);

            // A stomp fires automatically, instead of a plain swing, whenever the attack fires
            // while airborne, already falling, and aimed mostly downward.
            let stomp = !self.grounded && self.vy > 0.0 && dy > 0.5;
            if stomp {
                self.vy = self.vy.max(MARIO_STOMP_DIVE_VELOCITY_ROWS_PER_SEC / body_height);
            }

            self.attack_effect = Some(MarioAttackEffect { dx, dy, remaining: MARIO_ATTACK_EFFECT_DURATION, has_hit: false, heavy: heavy_held });
        }

        if let Some(effect) = &mut self.attack_effect {
            effect.remaining -= dt;
            if effect.remaining <= 0.0 {
                self.attack_effect = None;
            }
        }
    }
}

/// One static surface: `rect` in the same fractional space `MarioState::x`/`y` live in, `passable`
/// -- whether physics can ever stop against it at all. Per direct user design intent ("we should be
/// able to define collision per entity"): a platform is no longer always one solid rect -- it can be
/// a solid slab paired with a `passable` decorative underside, or in principle any other mix.
///
/// Deliberately, physics itself (`is_grounded`/`touching_wall`/`resolve_mario_collisions` below)
/// never takes a `MarioCollider` at all, only ever a plain `&[(f32, f32, f32, f32)]` rect slice --
/// the caller filters to `!passable` colliders before physics ever sees them, so physics doesn't
/// need to know this concept exists. Only `render::mario_body_text` takes the full list: it needs
/// both a rect *and* whether to paint it as light stone (solid) or dark stone (a pass-through
/// underside).
#[derive(Clone, Copy)]
pub struct MarioCollider {
    pub rect: (f32, f32, f32, f32),
    pub passable: bool,
}

/// The gap left between a solid slab's own `y1` and where its paired `passable` underside actually
/// starts, in the same fractional space `MarioCollider::rect` lives in -- reserved for the slab's
/// own visible "light" body (see `render::mario_body_text`'s own doc comment on why that body is
/// painted one row *below* the slab's collision row, never on it). `0.03` comfortably clears one
/// terminal row at the ~40-row reference height (`1/(40-1)` ≈ `0.0256`) with rounding margin, so the
/// light band and the dark underside land on genuinely distinct rows rather than risking collapsing
/// onto the same one at some terminal heights -- this session already hit exactly that class of
/// row-rounding fragility twice before with fractional-position math, so the margin here is
/// deliberate, not arbitrary.
///
/// **Real, live-reported bug this fixed**: an earlier version of this split painted the slab's own
/// collision row directly (no gap, no light band) -- from the player's own physical point of view
/// (gravity pulling everything down, viewed from the side), *any* row their own glyph shares with
/// platform material, light or dark, reads as floating inside solid rock, full stop, independent of
/// color. Compositing priority correctly shows the player's own glyph in that exact cell either way,
/// but neighboring cells on the same row still show platform texture, and that's what actually reads
/// wrong -- the fix isn't a color choice, it's leaving the whole row genuinely clear again, the same
/// principle the original "player renders embedded in platform" fix established, that this session's
/// intervening two-tone work briefly, mistakenly, moved away from.
pub const MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION: f32 = 0.03;
/// How tall the passable underside's own dark band is, once it starts (see
/// `MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION` for where it starts). `0.025` reads as roughly one
/// terminal row at the reference height, same reasoning as that constant. Exported so `mario-wasm`
/// (a real dependent of this crate) can build the same shape its own platform uses; native
/// `physics.rs` keeps its own independent copy of both these values (full disclosed duplication, see
/// this crate's own `lib.rs` doc comment), since it has no dependency on this crate at all.
pub const MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION: f32 = 0.025;

/// Where a newly-spawned (or respawned) player lands along the platform's own width, per direct
/// user request for a free-for-all brawl: "load at a semi-random location weighted heavier towards
/// the edge, with an even split. The effect is that any number of players could join, and they
/// would find a reasonable starting position." Pure and deterministic *given* its two random
/// inputs -- this crate has no entropy source of its own (native `physics.rs` and `mario-wasm` each
/// have a real one, system time and `js_sys::Math::random()` respectively, and neither belongs in
/// a crate meant to stay platform-agnostic) -- so callers supply two independent values in `[0.0,
/// 1.0)` and this does the actual (testable, seed-reproducible) placement math.
///
/// `side_random < 0.5` picks the left edge, otherwise the right -- an even split, not weighted
/// either direction. `position_random` is squared before use, which biases it toward `0.0`
/// (`t.powi(2)` compresses values toward the low end since `t < 1.0`), then scaled across just the
/// near half of the platform's width (edge to center) -- so a spawn is *always* at least as close
/// to its own edge as to the center, and clusters closer to the edge than a uniform pick would,
/// without ever gluing every spawn to the exact same pixel.
pub fn weighted_edge_x(platform_x0: f32, platform_x1: f32, side_random: f32, position_random: f32) -> f32 {
    let half_width = (platform_x1 - platform_x0) / 2.0;
    let edge_bias = position_random.clamp(0.0, 1.0).powi(2) * half_width;
    if side_random < 0.5 { platform_x0 + edge_bias } else { platform_x1 - edge_bias }
}

/// Whether `mario` currently rests on something: the floor, or the top of any platform. Checked
/// fresh every tick after collisions resolve, rather than tracked incrementally across the two
/// separate places that could affect it.
pub fn is_grounded(mario: &MarioState, colliders: &[(f32, f32, f32, f32)]) -> bool {
    mario.y >= MARIO_GROUND_Y
        || colliders
            .iter()
            .any(|&(x0, y0, x1, _y1)| mario.x >= x0 && mario.x <= x1 && (mario.y - y0).abs() < MARIO_GROUNDED_EPSILON)
}

/// See `MarioState::touching_wall`. Recomputed fresh every tick from the real screen edge (`x` at
/// 0.0 or 1.0) or the side of any platform. Only meaningful while airborne.
pub fn touching_wall(mario: &MarioState, colliders: &[(f32, f32, f32, f32)]) -> Option<f32> {
    if mario.grounded {
        return None;
    }
    if mario.x <= 0.0 {
        return Some(1.0);
    }
    if mario.x >= 1.0 {
        return Some(-1.0);
    }
    colliders.iter().find_map(|&(x0, y0, x1, y1)| {
        if mario.y < y0 || mario.y > y1 {
            return None;
        }
        if (mario.x - x0).abs() < MARIO_GROUNDED_EPSILON {
            Some(-1.0)
        } else if (mario.x - x1).abs() < MARIO_GROUNDED_EPSILON {
            Some(1.0)
        } else {
            None
        }
    })
}

/// Pushes `mario` back out of any `colliders` it ended up inside of (or swept clean through) after
/// `step` integrated its position for this tick. `prev_x`/`prev_y` are its position *before* that
/// integration. See the native example's own `physics.rs` for the full history of the swept-check
/// bug this fixed — copied verbatim here, not reworked.
pub fn resolve_mario_collisions(mario: &mut MarioState, prev_x: f32, prev_y: f32, colliders: &[(f32, f32, f32, f32)]) {
    for &(x0, y0, x1, y1) in colliders {
        // Falling onto the top face: was above it, ends at-or-below it.
        if mario.vy > 0.0 && prev_y <= y0 && mario.y >= y0 {
            let travel = mario.y - prev_y;
            let cross_x = if travel > f32::EPSILON { prev_x + (mario.x - prev_x) * ((y0 - prev_y) / travel) } else { prev_x };
            if cross_x >= x0 && cross_x <= x1 {
                mario.y = y0;
                mario.vy = 0.0;
                continue;
            }
        }
        // Jumping into the underside: was below it, ends at-or-above it.
        if mario.vy < 0.0 && prev_y >= y1 && mario.y <= y1 {
            let travel = prev_y - mario.y;
            let cross_x = if travel > f32::EPSILON { prev_x + (mario.x - prev_x) * ((prev_y - y1) / travel) } else { prev_x };
            if cross_x >= x0 && cross_x <= x1 {
                mario.y = y1;
                mario.vy = mario.vy.max(0.0);
                continue;
            }
        }

        if mario.x < x0 || mario.x > x1 || mario.y < y0 || mario.y > y1 {
            continue;
        }

        // Still overlapping after the swept checks above (e.g. genuinely moving sideways into the
        // platform's edge). Resolve whichever side is actually nearest now.
        let from_left = mario.x - x0;
        let from_right = x1 - mario.x;
        let from_top = mario.y - y0;
        let from_bottom = y1 - mario.y;
        let min = from_left.min(from_right).min(from_top).min(from_bottom);

        if min == from_top {
            mario.y = y0;
            mario.vy = 0.0;
        } else if min == from_bottom {
            mario.y = y1;
            mario.vy = mario.vy.max(0.0);
        } else if min == from_left {
            mario.x = x0;
            mario.vx = 0.0;
        } else {
            mario.x = x1;
            mario.vx = 0.0;
        }
    }
}

#[cfg(test)]
mod diagnostic_tests {
    use super::*;

    /// The standard play area every collision/render test in this module uses, paired with a
    /// platform collider — a small fixture so a test can just say "in this arena, with the player
    /// here, X should be true" instead of repeating the render call and row-scanning by hand.
    struct Arena {
        width: u16,
        height: u16,
        platform: (f32, f32, f32, f32),
    }

    impl Arena {
        fn with_platform(platform: (f32, f32, f32, f32)) -> Self {
            Arena { width: 80, height: 40, platform }
        }

        /// Solid-only, for physics -- matches how the real game filters colliders down before
        /// physics ever sees them (see `MarioCollider`'s own doc comment on why physics functions
        /// never take a `passable` flag at all).
        fn colliders(&self) -> [(f32, f32, f32, f32); 1] {
            [self.platform]
        }

        /// The full two-entity pair -- the solid slab plus its `passable` underside, with the same
        /// light-band gap between them -- for rendering. Mirrors exactly how the real game builds
        /// its own collider list for `mario_body_text`: same slab rect, same gap/underside sizing
        /// (`MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION`/`MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION`)
        /// physics.rs's own `spawn_platform` and mario-wasm's own platform setup use.
        fn render_colliders(&self) -> [MarioCollider; 2] {
            let (x0, _, x1, y1) = self.platform;
            let underside_y0 = y1 + MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION;
            [
                MarioCollider { rect: self.platform, passable: false },
                MarioCollider { rect: (x0, underside_y0, x1, underside_y0 + MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION), passable: true },
            ]
        }

        /// The exact terminal row the slab's own `y0` maps to -- the same `to_cell` formula
        /// `render::mario_body_text` uses internally, duplicated here so tests can assert row
        /// *relationships* (landed-on-top vs bonked-underside must render at different, correctly
        /// ordered rows relative to the slab) without needing to scan rendered text for glyphs, which
        /// can no longer tell light stone (the slab, painted at its own row by design since the
        /// two-tone split) from dark stone (the underside) apart just by character.
        fn slab_row(&self) -> usize {
            let denom = self.height.saturating_sub(1) as f32;
            (self.platform.1 * denom).round().clamp(0.0, denom) as usize
        }

        /// Advances `mario` by one real tick — `step` plus the same collision/grounding resolution
        /// `resolve_mario_collisions_and_grounding` runs in the real game — against this arena's
        /// platform. `move_input`/`jump_pressed`/`jump_held` are the only inputs tests here ever
        /// need to vary; everything else is neutral (no attack, no crouch, no stick-y).
        fn tick(&self, mario: &mut MarioState, dt: f32, move_input: f32, jump_pressed: bool, jump_held: bool) {
            let (prev_x, prev_y) = (mario.x, mario.y);
            mario.step(dt, move_input, 0.0, jump_pressed, jump_held, false, false, false, false, self.height as f32, self.width as f32);
            resolve_mario_collisions(mario, prev_x, prev_y, &self.colliders());
            mario.grounded = is_grounded(mario, &self.colliders());
            if mario.grounded {
                mario.jumps_used = 0;
            }
        }

        /// Renders `mario` alone in this arena and returns (the row its own glyph landed on, every
        /// row containing platform texture) — the two things every render test here actually cares
        /// about. `None` if the player's glyph isn't in the frame at all (fell below the visible
        /// area, say), a real and expected outcome for some trajectories, not a test error.
        fn render_rows(&self, mario: MarioState) -> Option<(usize, Vec<usize>)> {
            let backdrop_rows = vec![" ".repeat(self.width as usize); self.height as usize];
            let sprites = [(mario, mario_player_color(0), 0usize)];
            let frame = crate::render::mario_body_text(&backdrop_rows, &sprites, &[], None, self.width, self.height, &self.render_colliders());
            let rows: Vec<&str> = frame.split('\n').collect();
            let player_row = rows.iter().position(|row| row.contains(crate::render::mario_player_flair(0)))?;
            // Checks against the real `render::STONE_TEXTURE_CHARS` set, not a hardcoded duplicate
            // -- an earlier version of this test hardcoded a literal `'='` check instead, and went
            // silently stale (passing vacuously on an empty match) the moment that set's actual
            // characters changed. Exactly the kind of test/implementation drift this crate's own
            // tests exist to catch, not fall victim to.
            let platform_texture_rows = rows
                .iter()
                .enumerate()
                .filter(|(_, row)| row.chars().any(|c| crate::render::STONE_TEXTURE_CHARS.contains(&c)))
                .map(|(i, _)| i)
                .collect();
            Some((player_row, platform_texture_rows))
        }
    }

    fn attack_dy(mut mario: MarioState, stick_y: f32) -> Option<f32> {
        mario.step(1.0 / 60.0, 0.0, stick_y, false, false, true, false, false, false, 40.0, 80.0);
        mario.attack_effect.map(|effect| effect.dy)
    }

    #[test]
    fn attack_direction_on_ground_vs_platform() {
        let on_ground = MarioState::default();
        assert_eq!(on_ground.y, MARIO_GROUND_Y);
        assert!(on_ground.grounded);

        let mut on_platform = MarioState::default();
        on_platform.y = 0.6773; // typical platform top y0
        on_platform.grounded = true;

        let ground_up = attack_dy(on_ground, -1.0);
        let ground_down = attack_dy(on_ground, 1.0);
        let platform_up = attack_dy(on_platform, -1.0);
        let platform_down = attack_dy(on_platform, 1.0);

        println!("ground_up dy={ground_up:?} ground_down dy={ground_down:?} platform_up dy={platform_up:?} platform_down dy={platform_down:?}");

        assert_eq!(ground_up, Some(-1.0), "up should always give dy=-1.0 (up on screen), regardless of surface");
        assert_eq!(ground_down, Some(1.0), "down should always give dy=1.0 (down on screen), regardless of surface");
        assert_eq!(platform_up, Some(-1.0), "up should always give dy=-1.0 (up on screen), regardless of surface");
        assert_eq!(platform_down, Some(1.0), "down should always give dy=1.0 (down on screen), regardless of surface");
    }

    /// Real multi-tick simulation of the exact scenario reported live: jump from the ground into
    /// the platform's underside, then land on top of it after falling back down. `PLATFORM` mirrors
    /// `mario-wasm`'s own `platform_rect()`.
    #[test]
    fn jump_into_platform_underside_then_land_on_top() {
        let arena = Arena::with_platform((0.3, 0.6573, 0.7, 0.6773));
        let dt = 1.0 / 60.0;
        let mut mario = MarioState::default();

        let mut min_y = mario.y;
        let mut ever_passed_through = false;
        // Jump, holding it the whole time (max height), for 3 seconds -- long enough to clear a
        // full arc and land back down regardless of tuning.
        for i in 0..180 {
            arena.tick(&mut mario, dt, 0.0, i == 0, true);
            min_y = min_y.min(mario.y);
            // "Passed through" means ending up strictly inside the platform's y-range with x inside
            // its span -- exactly what solid-collision resolution should make impossible.
            let (x0, y0, x1, y1) = arena.platform;
            if mario.y > y0 && mario.y < y1 && mario.x >= x0 && mario.x <= x1 {
                ever_passed_through = true;
            }
        }

        println!("min_y={min_y} final_y={} final_grounded={} ever_passed_through={ever_passed_through}", mario.y, mario.grounded);

        let (_, platform_y0, _, platform_y1) = arena.platform;
        assert!(!ever_passed_through, "player should never end a tick strictly inside the platform's solid body");
        assert!(min_y >= platform_y1, "a jump should bump off the platform's underside (y >= {platform_y1}), not tunnel through it -- got min_y={min_y}");
        // After 3s the jump should have long since landed somewhere: either back on the true
        // ground, or resting on the platform's own top surface.
        assert!(
            (mario.y - MARIO_GROUND_Y).abs() < 0.001 || (mario.y - platform_y0).abs() < 0.001,
            "should have landed on the ground ({MARIO_GROUND_Y}) or the platform's top ({platform_y0}), got y={}",
            mario.y
        );
    }

    /// Whether landing on the platform actually *renders* as standing on top of it, not embedded
    /// in it -- the live-reported visual bug (a screenshot, described plainly: "the red dot is
    /// inside the brown shape"). Real integration test, not a hand-rolled reimplementation of the
    /// render math: calls the actual `mario_body_text` the game draws with, then inspects the real
    /// output string for which row the player's own glyph lands on.
    ///
    /// Rewritten against `slab_row()` rather than scanning for platform-texture glyphs, per the
    /// two-tone entity split: the slab's own row is now *deliberately* painted (light stone) rather
    /// than left blank, so "the player's row is a textured row" is no longer itself the bug --
    /// that's expected now, and sprite-compositing priority (checked before platform fill) is what
    /// keeps it reading as "standing on" rather than "inside." What still matters, and is still
    /// checked here: landing renders exactly *at* the slab's own row, never below it into the dark
    /// underside band.
    #[test]
    fn landed_on_platform_renders_above_its_texture_not_inside_it() {
        let arena = Arena::with_platform((0.3, 0.6573, 0.7, 0.6773));
        let mut mario = MarioState::default();
        mario.x = 0.5;
        mario.y = arena.platform.1; // resting exactly on the platform's top surface, as a real landing leaves it
        mario.grounded = true;

        let (player_row, platform_texture_rows) = arena.render_rows(mario).expect("player glyph should be in the rendered frame");
        let slab_row = arena.slab_row();
        println!("player_row={player_row} slab_row={slab_row} platform_texture_rows={platform_texture_rows:?}");

        assert_eq!(player_row, slab_row, "landing on top should render exactly at the slab's own row ({slab_row}), not the dark underside band below it");
    }

    /// The other half of the same bug, per direct live follow-up: "my head should collide with the
    /// bottom of the gray shape [...] currently it passes through to the top edge and stops there."
    /// Same real-integration approach as the landing-on-top test above: calls the actual
    /// `mario_body_text`, this time with `mario.y` at the collider's *underside* (`y1`) rather than
    /// its top (`y0`) -- exactly what `resolve_mario_collisions` leaves a player at after bonking a
    /// platform from below.
    ///
    /// Rewritten against `slab_row()`, same reasoning as the landing test above: what matters is
    /// that bonking renders strictly *below* the slab's own row (into the dark underside band,
    /// which is precisely what that band is there to receive), not "below its texture" -- the slab
    /// itself is textured now too.
    #[test]
    fn bonked_underside_renders_below_its_texture_not_coinciding_with_landing_on_top() {
        let arena = Arena::with_platform((0.3, 0.6573, 0.7, 0.6773));
        let mut mario = MarioState::default();
        mario.x = 0.5;
        mario.y = arena.platform.3; // resting exactly on the underside, as a real bonk leaves it (RECT.3 == y1)
        mario.grounded = false;

        let (player_row, platform_texture_rows) = arena.render_rows(mario).expect("player glyph should be in the rendered frame");
        let slab_row = arena.slab_row();
        println!("player_row={player_row} slab_row={slab_row} platform_texture_rows={platform_texture_rows:?}");

        assert!(player_row > slab_row, "bonking the underside should render strictly below the slab's own row ({slab_row}), not coincide with it (landing-on-top's row) -- got {player_row}");
    }

    /// Real, live-reported follow-up to the fix above: "I see the avatar flash for a second at the
    /// bottom edge of the platform [...] but I can still break into the box every single time (it
    /// just flickers [...] for a single frame or so." An earlier version of the underside fix only
    /// checked the exact instant `y == y1`; every frame after, as gravity carries the player away,
    /// it fell back to the old (wrong) behavior. This runs a real multi-tick post-bonk trajectory
    /// (actual `step`/`resolve_mario_collisions`, not a single hand-placed `y`) and checks *every*
    /// frame's real rendered output, not just the first one -- exactly what the single-instant
    /// version of this test couldn't have caught.
    ///
    /// Rewritten against `slab_row()`, same reasoning as the two tests above.
    #[test]
    fn bonked_underside_stays_below_texture_for_every_frame_until_clear() {
        let arena = Arena::with_platform((0.3, 0.6573, 0.7, 0.6773));
        let dt = 1.0 / 60.0;
        let slab_row = arena.slab_row();

        let mut mario = MarioState::default();
        mario.x = 0.5;
        mario.y = arena.platform.3; // exactly the underside, as a real bonk leaves it
        mario.vy = 0.0;
        mario.grounded = false;

        // 30 real frames (0.5s) of actual physics -- gravity carries the player away from the
        // underside over multiple ticks, not instantly.
        for _ in 0..30 {
            arena.tick(&mut mario, dt, 0.0, false, false);

            // Player fell off the bottom of the visible area this tick -- fine, nothing left to
            // check against this collider.
            let Some((player_row, _)) = arena.render_rows(mario) else { continue };
            assert!(
                player_row > slab_row,
                "y={} player_row={player_row} should stay strictly below the slab's own row ({slab_row}) every frame -- the exact 'flickers once then breaks back in' bug",
                mario.y
            );
        }
    }
}
