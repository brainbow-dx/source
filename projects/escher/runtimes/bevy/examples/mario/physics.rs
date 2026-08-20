//! Physics, input, and combat for one player square per connected gamepad: gravity, jumps
//! (including a double jump and a wall kick), an attack with a cooldown and a mild anti-spam
//! penalty, lives and respawn, and gamepad-ownership arbitration so two instances on the same
//! machine never both drive the same physical controller.

use std::sync::LazyLock;
use std::time::Duration;

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Commands;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::ecs::message::MessageWriter;
use bevy::time::Time;

use crate::persistence::PersistenceWrite;
use crate::relay::PositionPacket;
use crate::render::apply_cheat;
use crate::render::CHEAT_ENTRIES;
use crate::GameState;

/// One player's live physics and combat state. `x`/`y` are fractions of the play area's own
/// width/height (0.0-1.0), not raw cell coordinates, so `step` never needs to know the terminal's
/// current size and a player stays proportionally in the same spot across a resize.
#[derive(Clone, Copy)]
pub struct MarioState {
    pub x: f32,
    pub y: f32,
    /// `x`/`y` as of the start of this tick's `step`, before gravity/input moved them — what
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
    /// Kills landed this session by this player against locally simulated players. Remote players
    /// aren't included: this instance has no authority over a remote player's state, so a cross-
    /// instance hit needs its own networked damage event, not local position math. Not implemented
    /// yet.
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
/// roster order: Mario (red), Luigi (green), Peach (pink), Toad (white with a red accent — an off-
/// white here rather than pure white so it still reads as a distinct color, not "no color"). Not
/// yet player-chosen (see `render::PLAYER_FLAIRS`'s own doc comment) — every player index maps to
/// this fixed order until a real picker exists.
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
/// Pressure-sensitive jumping, implemented the way most platformers do it: not by measuring press
/// duration up front, but by applying much stronger gravity the instant the button is released
/// while still ascending. Holding through the whole ascent never triggers this, so a held jump
/// reaches full height, while releasing early cuts the ascent short.
pub const MARIO_JUMP_CUT_GRAVITY_MULTIPLIER: f32 = 2.5;
/// One extra jump while airborne, noticeably smaller than the primary jump.
pub const MARIO_DOUBLE_JUMP_VELOCITY_ROWS_PER_SEC: f32 = -38.0;
pub const MARIO_MAX_JUMPS: u8 = 2;
/// How fast a wall kick snaps `vx` toward the chosen direction, faster than ordinary movement so it
/// reads as a decisive push-off rather than a normal jump that happens to also move sideways.
pub const MARIO_WALL_KICK_SPEED_MULTIPLIER: f32 = 1.5;
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
/// Pressing down in the air is already spoken for, aiming a downward stomp attack instead.
pub const MARIO_CROUCH_SPEED_MULTIPLIER: f32 = 0.45;
pub const MARIO_DUST_EFFECT_DURATION: f32 = 0.35;
pub const MARIO_ATTACK_BUTTON: bevy::input::gamepad::GamepadButton = bevy::input::gamepad::GamepadButton::East;
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
/// A hit only counts within roughly this cone in front of the swing's own direction, as the dot
/// product of the unit vector toward the target against the swing's unit direction. `0.35` is
/// roughly a 70 degree half-angle: a real forward cone, not a hitbox radiating outward from center.
pub const MARIO_ATTACK_DIRECTION_COS_THRESHOLD: f32 = 0.35;
/// The extra downward dive velocity a stomp snaps to the instant it triggers. A stomp doesn't hit
/// for more than a plain swing: every connected hit is already a full kill, so a stomp is
/// distinguished only by this dive, its trigger condition, and its glyph.
pub const MARIO_STOMP_DIVE_VELOCITY_ROWS_PER_SEC: f32 = 70.0;
pub const MARIO_STARTING_LIVES: u8 = 3;
pub const MARIO_HIT_RUMBLE_TARGET_INTENSITY: bevy::input::gamepad::GamepadRumbleIntensity = bevy::input::gamepad::GamepadRumbleIntensity::STRONG_MAX;
pub const MARIO_HIT_RUMBLE_TARGET_DURATION: Duration = Duration::from_millis(220);
pub const MARIO_HIT_RUMBLE_ATTACKER_INTENSITY: bevy::input::gamepad::GamepadRumbleIntensity = bevy::input::gamepad::GamepadRumbleIntensity::WEAK_MAX;
pub const MARIO_HIT_RUMBLE_ATTACKER_DURATION: Duration = Duration::from_millis(90);
/// How long a death with lives remaining waits before respawning.
pub const MARIO_RESPAWN_SECONDS: f32 = 5.0;
pub const MARIO_DEATH_EFFECT_DURATION: f32 = 0.6;
/// `y` at rest: the play area's very last row, immediately above the footer.
pub const MARIO_GROUND_Y: f32 = 1.0;
/// Vertical tolerance for "resting exactly on a platform's top edge". Comparing floats for exact
/// equality is fragile, so this allows the tiny drift a collision or gravity step can introduce
/// before the next grounded check runs.
const MARIO_GROUNDED_EPSILON: f32 = 0.002;

impl MarioState {
    /// Advances the simulation by `dt` seconds. Pure: no Bevy types, no I/O, so the gamepad-reading
    /// system stays a thin adapter over this and this itself stays independently testable.
    /// `move_input`: -1.0 (full left) to 1.0 (full right). `jump_held`: still down this tick,
    /// separate from the press edge so a held-vs-tapped button can be told apart at all, which is
    /// what pressure-sensitive jumping needs. `body_height`/`body_width`: the play area's current
    /// height/width in rows/columns. `crouch_held`: only actually crouches while `grounded`,
    /// pressing down in the air still just aims a downward stomp.
    #[allow(clippy::too_many_arguments)]
    pub fn step(
        &mut self,
        dt: f32,
        move_input: f32,
        stick_y: f32,
        jump_pressed: bool,
        jump_held: bool,
        attack_pressed: bool,
        crouch_held: bool,
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

            let stick_active = move_input.abs() > MARIO_STICK_DEADZONE || stick_y.abs() > MARIO_STICK_DEADZONE;
            let (dx, dy) = if stick_active {
                let length = (move_input * move_input + stick_y * stick_y).sqrt().max(f32::EPSILON);
                (move_input / length, stick_y / length)
            } else {
                (self.facing, 0.0)
            };

            // A stomp fires automatically, instead of a plain swing, whenever the attack fires
            // while airborne, already falling, and aimed mostly downward.
            let stomp = !self.grounded && self.vy > 0.0 && dy > 0.5;
            if stomp {
                self.vy = self.vy.max(MARIO_STOMP_DIVE_VELOCITY_ROWS_PER_SEC / body_height);
            }

            self.attack_effect = Some(MarioAttackEffect { dx, dy, remaining: MARIO_ATTACK_EFFECT_DURATION, has_hit: false });
        }

        if let Some(effect) = &mut self.attack_effect {
            effect.remaining -= dt;
            if effect.remaining <= 0.0 {
                self.attack_effect = None;
            }
        }
    }
}

/// Whether `mario` currently rests on something: the floor, or the top of any platform. Checked
/// fresh every tick after collisions resolve, rather than tracked incrementally across the two
/// separate places that could affect it.
fn is_grounded(mario: &MarioState, colliders: &[(f32, f32, f32, f32)]) -> bool {
    mario.y >= MARIO_GROUND_Y
        || colliders
            .iter()
            .any(|&(x0, y0, x1, _y1)| mario.x >= x0 && mario.x <= x1 && (mario.y - y0).abs() < MARIO_GROUNDED_EPSILON)
}

/// See `MarioState::touching_wall`. Recomputed fresh every tick from the real screen edge (`x` at
/// 0.0 or 1.0) or the side of any platform. Only meaningful while airborne.
fn touching_wall(mario: &MarioState, colliders: &[(f32, f32, f32, f32)]) -> Option<f32> {
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

/// A static platform, `(x0, y0, x1, y1)` in the same 0.0-1.0 fractional space `MarioState::x`/`y`
/// live in. Spawned once at startup and never moved.
#[derive(Component)]
pub struct MarioCollider {
    pub rect: (f32, f32, f32, f32),
}

/// Pushes `mario` back out of any `colliders` it ended up inside of (or swept clean through) after
/// `step` integrated its position for this tick. `prev_x`/`prev_y` are its position *before* that
/// integration (`MarioState::prev_x`/`prev_y`, stamped at the top of `step`).
///
/// Real bug, found live: the original version of this picked a resolution axis purely by *minimum
/// penetration depth* of wherever `mario` ended up this tick, with no notion of which direction it
/// was actually traveling. Against a thin platform (the single static one here is only ~0.03 of
/// the play area tall), a fast fall's single-tick step routinely integrates `y` to somewhere in the
/// *lower* half of that thin box — at which point "least penetration" is the *bottom* face, so the
/// old code shoved the player down and out through the floor of the very platform they were
/// falling onto, reading as "I land in it, then fall through" (exactly the user's own report).
/// Worse, if a fast enough fall's single tick skipped the box's thin `y` range entirely, no
/// overlap was ever detected at all, and the player fell straight through with no resolution — the
/// "falls through inconsistently" half of the same report.
///
/// Fixed with a proper swept check, done first and unconditionally on `y` regardless of where the
/// tick's integration left `mario` relative to the box: if the straight line from `(prev_x,
/// prev_y)` to `(mario.x, mario.y)` crosses the platform's top or bottom face while inside its `x`
/// span, that's a real landing/head-bump this tick had, whether or not the final position also
/// happens to still be inside the box. Left/right still resolve from the (now already
/// top/bottom-corrected) final overlap, since lateral speed is never anywhere near enough to tunnel
/// a whole platform width in one tick the way a fall can tunnel its thin height.
///
/// Solid on every face, per the user's own explicit call after trying a one-way (solid-from-above-
/// only) version: a player jumping up into it should bump off the underside exactly like landing
/// on top, not pass through. That one-way detour was chasing the wrong cause anyway — the "avatar
/// looks embedded in the platform" report turned out to be `render.rs` never giving the platform
/// any visual thickness at all (a single character row, indistinguishable from the ground), fixed
/// there instead; the collision behavior here was never actually the problem.
fn resolve_mario_collisions(mario: &mut MarioState, prev_x: f32, prev_y: f32, colliders: &[(f32, f32, f32, f32)]) {
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
        // platform's edge) — resolve whichever side is actually nearest now.
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

/// This machine's hostname, resolved once. Gamepad-ownership arbitration scopes to it so two
/// different physical controllers of the same model on two different machines never compete for
/// ownership, only ones that could plausibly be the exact same USB device.
static MACHINE_ID: LazyLock<String> =
    LazyLock::new(|| hostname::get().ok().and_then(|name| name.into_string().ok()).unwrap_or_else(|| "unknown-host".to_string()));

/// Identifies a controller model (vendor/product id), not a physical unit: there's no serial
/// number anywhere in this stack. Two identical-model controllers on one machine are genuinely
/// indistinguishable by hardware alone. `MACHINE_ID` is prefixed in so two different people on two
/// different machines who happen to own the same model don't collide on this id the way two
/// identical controllers on one machine would.
pub fn gamepad_candidate_id(pad: &bevy::input::gamepad::Gamepad) -> String {
    format!("{}:{:04x}:{:04x}", *MACHINE_ID, pad.vendor_id().unwrap_or(0), pad.product_id().unwrap_or(0))
}

/// Steps one `MarioState` per connected gamepad. `bevy_gilrs` gives each connected controller its
/// own ECS entity carrying a `Gamepad` component, so "one sprite per controller" falls out of
/// iterating the query fully. `GameState::mario` is keyed by that same entity: a newly seen one
/// gets a fresh `MarioState::default()`, and one no longer in the query (controller unplugged) is
/// dropped. A physical controller that disconnects and reconnects gets a new entity and so a fresh,
/// reset sprite: `bevy_gilrs` doesn't preserve identity across that gap.
///
/// `dt` is clamped to a small fixed range regardless of how long actually elapsed, so a sparse tick
/// produces one bounded hop rather than a teleport or falling through the floor.
pub fn update_mario_physics(
    state: Res<GameState>,
    gamepads: Query<(Entity, &bevy::input::gamepad::Gamepad)>,
    colliders: Query<&MarioCollider>,
    time: Res<Time>,
    mut rumble: MessageWriter<bevy::input::gamepad::GamepadRumbleRequest>,
) {
    let dt = time.delta_secs().clamp(1.0 / 60.0, 0.1);

    // Ghosts freeze too while the pause menu is open: see `GameState::paused_accumulated`.
    if *state.cheat_menu_open.read() {
        *state.paused_accumulated.write() += Duration::from_secs_f32(dt);
    }

    let body_height = state.body_rect_seen.read().map(|rect| rect.height as f32).filter(|&height| height > 0.0).unwrap_or(40.0);
    let body_width = state.body_rect_seen.read().map(|rect| rect.width as f32).filter(|&width| width > 0.0).unwrap_or(100.0);

    let collider_rects: Vec<(f32, f32, f32, f32)> = colliders.iter().map(|collider| collider.rect).collect();

    // Published for the ownership-reconciliation task regardless of ownership: it needs to know
    // about every gamepad this instance can physically see, not just the ones it currently
    // controls.
    *state.visible_gamepad_candidates.write() = gamepads.iter().map(|(_, pad)| gamepad_candidate_id(pad)).collect();

    // No sqld connection, or the first tick or two after connecting before the first sync round
    // lands, means nothing to reconcile against: trust the local view entirely.
    let reconciling = state.persistence.read().is_some();
    let owned = state.gamepad_owned_by_me.read();
    let is_owned = |pad: &bevy::input::gamepad::Gamepad| !reconciling || owned.contains(&gamepad_candidate_id(pad));

    let mut mario = state.mario.write();

    mario.retain(|(entity, ..)| gamepads.get(*entity).is_ok_and(|(_, pad)| is_owned(pad)));

    for (entity, pad) in gamepads.iter() {
        if !is_owned(pad) {
            continue;
        }

        // A deadzone on the stick specifically, not the d-pad: real analog sticks can report a
        // small nonzero value at rest purely from hardware wear, which without this reads as
        // constant unintended drift.
        let stick_x = pad.left_stick().x;
        let move_input = if stick_x.abs() < MARIO_STICK_DEADZONE { 0.0 } else { stick_x } + pad.dpad().x;
        // Flipped from the stick's own "up is positive" convention to `MarioState::y`'s "down is
        // positive" one, so pushing the stick down aims an attack toward increasing `y`.
        let stick_y_raw = pad.left_stick().y;
        let attack_stick_y = if stick_y_raw.abs() < MARIO_STICK_DEADZONE { 0.0 } else { -stick_y_raw };
        let crouch_held = stick_y_raw < -MARIO_STICK_DEADZONE || pad.dpad().y < -MARIO_STICK_DEADZONE;
        let jump_pressed = pad.just_pressed(bevy::input::gamepad::GamepadButton::South);
        let jump_held = pad.pressed(bevy::input::gamepad::GamepadButton::South);
        let attack_pressed = pad.just_pressed(MARIO_ATTACK_BUTTON);

        // The pause menu opens on a real gamepad Start button. Any one of this instance's owned
        // gamepads can open or close it.
        if pad.just_pressed(bevy::input::gamepad::GamepadButton::Start) {
            let mut open = state.cheat_menu_open.write();
            *open = !*open;
            *state.cheat_menu_selected.write() = 0;
        }

        // While the menu is open, every gamepad's input drives it instead of a player: players and
        // ghosts freeze. D-pad up/down navigates, South confirms. `continue` skips this gamepad's
        // own player entirely for the tick, which is what "freeze" means for a player (ghosts
        // freeze via the wall-clock accumulator above instead).
        if *state.cheat_menu_open.read() {
            if pad.just_pressed(bevy::input::gamepad::GamepadButton::DPadUp) {
                let mut selected = state.cheat_menu_selected.write();
                *selected = selected.checked_sub(1).unwrap_or(CHEAT_ENTRIES.len() - 1);
            }
            if pad.just_pressed(bevy::input::gamepad::GamepadButton::DPadDown) {
                let mut selected = state.cheat_menu_selected.write();
                *selected = (*selected + 1) % CHEAT_ENTRIES.len();
            }
            if pad.just_pressed(bevy::input::gamepad::GamepadButton::South) {
                apply_cheat(&mut mario, *state.cheat_menu_selected.read());
            }
            continue;
        }

        match mario.iter_mut().find(|(existing, ..)| *existing == entity) {
            Some((_, _, mario_state)) if mario_state.alive => {
                mario_state.step(dt, move_input, attack_stick_y, jump_pressed, jump_held, attack_pressed, crouch_held, body_height, body_width)
            }
            // Dead with lives left: just counts down to a respawn. Dead with no lives left
            // (`respawn_remaining` already `None`) falls through to this arm and does nothing.
            Some((_, _, mario_state)) => {
                if let Some(remaining) = mario_state.respawn_remaining.as_mut() {
                    *remaining -= dt;
                    if *remaining <= 0.0 {
                        mario_state.respawn_remaining = None;
                        mario_state.alive = true;
                        mario_state.x = 0.5;
                        mario_state.y = MARIO_GROUND_Y;
                        mario_state.vx = 0.0;
                        mario_state.vy = 0.0;
                        mario_state.jumps_used = 0;
                    }
                }
                if let Some(effect) = &mut mario_state.death_effect {
                    effect.remaining -= dt;
                    if effect.remaining <= 0.0 {
                        mario_state.death_effect = None;
                    }
                }
            }
            None => mario.push((entity, gamepad_candidate_id(pad), MarioState::default())),
        }
    }

    // Hit detection: any player whose swing is still live and hasn't already landed a hit checks
    // every other local player for contact. Cross-instance hits aren't handled here: this instance
    // has no authority over a remote player's state, and their networked position lags real time
    // slightly regardless. That needs a real damage-event message over the relay's data channel,
    // not local-only position math.
    for attacker_index in 0..mario.len() {
        if !mario[attacker_index].2.alive {
            continue;
        }
        let Some(effect) = mario[attacker_index].2.attack_effect else { continue };
        if effect.has_hit {
            continue;
        }

        let (attacker_x, attacker_y) = (mario[attacker_index].2.x, mario[attacker_index].2.y);
        let hit_target_index = mario
            .iter()
            .enumerate()
            .find(|&(target_index, (_, _, target))| {
                if target_index == attacker_index || !target.alive {
                    return false;
                }
                let (delta_x, delta_y) = (target.x - attacker_x, target.y - attacker_y);
                let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();
                if distance > MARIO_ATTACK_REACH || distance <= f32::EPSILON {
                    return false;
                }
                let alignment = (delta_x / distance) * effect.dx + (delta_y / distance) * effect.dy;
                alignment > MARIO_ATTACK_DIRECTION_COS_THRESHOLD
            })
            .map(|(target_index, _)| target_index);

        let Some(target_index) = hit_target_index else { continue };

        mario[attacker_index].2.attack_effect.as_mut().unwrap().has_hit = true;
        mario[attacker_index].2.kills += 1;

        // The target gets the heavier rumble, since they're the one getting destroyed. The
        // attacker gets a lighter confirmation tap that the swing landed.
        rumble.write(bevy::input::gamepad::GamepadRumbleRequest::Add {
            gamepad: mario[target_index].0,
            intensity: MARIO_HIT_RUMBLE_TARGET_INTENSITY,
            duration: MARIO_HIT_RUMBLE_TARGET_DURATION,
        });
        rumble.write(bevy::input::gamepad::GamepadRumbleRequest::Add {
            gamepad: mario[attacker_index].0,
            intensity: MARIO_HIT_RUMBLE_ATTACKER_INTENSITY,
            duration: MARIO_HIT_RUMBLE_ATTACKER_DURATION,
        });

        // Every connected hit is a full kill: there's no partial damage scale.
        let (death_x, death_y) = (mario[target_index].2.x, mario[target_index].2.y);
        mario[target_index].2.alive = false;
        mario[target_index].2.death_effect =
            Some(MarioDeathEffect { x: death_x, y: death_y, remaining: MARIO_DEATH_EFFECT_DURATION, burst_dx: -effect.dx, burst_dy: -effect.dy });
        mario[target_index].2.lives = mario[target_index].2.lives.saturating_sub(1);
        mario[target_index].2.respawn_remaining = if mario[target_index].2.lives > 0 { Some(MARIO_RESPAWN_SECONDS) } else { None };

        // Every lost life becomes a permanent floating ghost, regardless of whether this death
        // exhausted this player's lives.
        let target_candidate_id = mario[target_index].1.clone();
        let (target_name, target_color) = {
            let connected_players = state.connected_players.read();
            connected_players
                .iter()
                .find(|(id, ..)| *id == target_candidate_id)
                .map(|(_, name, color)| (name.clone(), *color))
                .unwrap_or_else(|| (target_candidate_id.clone(), mario_player_color(target_index)))
        };
        if let Some(sender) = state.persistence_writes.read().as_ref() {
            let _ = sender.send(PersistenceWrite::Defeat { candidate_id: target_candidate_id, name: target_name, color: target_color });
        }
    }

    for (_, _, mario_state) in mario.iter_mut() {
        let (prev_x, prev_y) = (mario_state.prev_x, mario_state.prev_y);
        resolve_mario_collisions(mario_state, prev_x, prev_y, &collider_rects);
        mario_state.grounded = is_grounded(mario_state, &collider_rects);
        // Landing, on the real floor or a platform, refills both jumps.
        if mario_state.grounded {
            mario_state.jumps_used = 0;
        }
        mario_state.touching_wall = touching_wall(mario_state, &collider_rects);
    }

    // Refreshed every tick regardless of whether anything moved: the relay's own send loop reads
    // this on a fixed interval, and a future authoritative server needs a steady stream, not
    // change-detection.
    *state.local_mario_snapshot.write() = mario
        .iter()
        .map(|(_, candidate_id, mario_state)| PositionPacket {
            candidate_id: candidate_id.clone(),
            seq: 0,
            x: mario_state.x,
            y: mario_state.y,
            vx: mario_state.vx,
            vy: mario_state.vy,
            grounded: mario_state.grounded,
            jumps_used: mario_state.jumps_used,
        })
        .collect();
}

/// Spawns the single static platform this example plays on, once, at startup. Placed relative to
/// the ground (`MARIO_GROUND_Y`), not a fixed-looking "reasonable" middle height: max jump height
/// is a *fraction of the play area's height* (`MARIO_JUMP_VELOCITY_ROWS_PER_SEC.powi(2) / (2.0 *
/// MARIO_GRAVITY_ROWS_PER_SEC2 * body_height)` rows canceling out), so a fixed fractional platform
/// height only reads as "just under one jump" on the roughly 40-row terminal these values are
/// tuned against — computed below from that same reference height, then pulled in slightly (the
/// `0.85` margin) so a full-height jump reliably clears it rather than just barely brushing the
/// underside. On an unusually tall terminal the platform sits proportionally lower and easier to
/// reach, not harder — the opposite failure from a fixed offset that's too high to jump to.
const MARIO_PLATFORM_REFERENCE_HEIGHT_ROWS: f32 = 40.0;
const MARIO_PLATFORM_JUMP_HEIGHT_MARGIN: f32 = 0.85;

pub fn spawn_platform(mut commands: Commands) {
    let jump_apex_rows =
        (MARIO_JUMP_VELOCITY_ROWS_PER_SEC * MARIO_JUMP_VELOCITY_ROWS_PER_SEC) / (2.0 * MARIO_GRAVITY_ROWS_PER_SEC2);
    let height_above_ground =
        (jump_apex_rows / MARIO_PLATFORM_REFERENCE_HEIGHT_ROWS) * MARIO_PLATFORM_JUMP_HEIGHT_MARGIN;
    let y1 = MARIO_GROUND_Y - height_above_ground;
    let y0 = y1 - 0.02;
    commands.spawn(MarioCollider { rect: (0.3, y0, 0.7, y1) });
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATFORM: (f32, f32, f32, f32) = (0.3, 0.91, 0.7, 0.93);

    #[test]
    fn fast_fall_lands_on_a_thin_platform() {
        let mut mario = MarioState { x: 0.5, prev_x: 0.5, y: 0.905, prev_y: 0.905, vy: 1.0, ..MarioState::default() };
        // One tick's worth of a fast fall, integrated exactly the way `step` does, deliberately
        // large enough to jump clean over the platform's own 0.02-tall span in a single step —
        // the exact scenario the old min-penetration resolver got wrong (see `resolve_mario_
        // collisions`'s own doc comment).
        mario.y += mario.vy * 0.1;
        assert!(mario.y > PLATFORM.3, "test setup should overshoot clean past the platform, got y={}", mario.y);

        let (prev_x, prev_y) = (mario.prev_x, mario.prev_y);
        resolve_mario_collisions(&mut mario, prev_x, prev_y, &[PLATFORM]);

        assert_eq!(mario.y, PLATFORM.1, "should land exactly on the platform's top surface, not tunnel through it");
        assert_eq!(mario.vy, 0.0);
    }

    #[test]
    fn landing_resolves_to_the_platforms_top() {
        // Ends the tick already inside the thin box, closer to its bottom face than its top —
        // exactly the case the old "least penetration" logic got backwards.
        let mut mario = MarioState { x: 0.5, prev_x: 0.5, y: 0.929, prev_y: 0.88, vy: 0.5, ..MarioState::default() };
        let (prev_x, prev_y) = (mario.prev_x, mario.prev_y);
        resolve_mario_collisions(&mut mario, prev_x, prev_y, &[PLATFORM]);
        assert_eq!(mario.y, PLATFORM.1, "a falling player landing inside a thin platform must resolve to its TOP, not its bottom");
    }

    #[test]
    fn jumping_up_through_the_platform_stops_at_its_underside() {
        let mut mario = MarioState { x: 0.5, prev_x: 0.5, y: 0.95, prev_y: 0.95, vy: -0.5, ..MarioState::default() };
        mario.y += mario.vy * 0.1;
        assert!(mario.y < PLATFORM.1, "test setup should overshoot clean past the platform going up, got y={}", mario.y);

        let (prev_x, prev_y) = (mario.prev_x, mario.prev_y);
        resolve_mario_collisions(&mut mario, prev_x, prev_y, &[PLATFORM]);

        assert_eq!(mario.y, PLATFORM.3, "should stop exactly at the platform's underside");
    }
}
