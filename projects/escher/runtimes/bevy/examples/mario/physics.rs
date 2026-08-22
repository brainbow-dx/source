//! Physics, input, and combat for one player square per connected gamepad: gravity, jumps
//! (including a double jump and a wall kick), an attack with a cooldown and a mild anti-spam
//! penalty, lives and respawn, and gamepad-ownership arbitration so two instances on the same
//! machine never both drive the same physical controller.

use std::sync::LazyLock;
use std::time::Duration;

use bevy::ecs::component::Component;
use bevy::ecs::entity::Entity;
use bevy::ecs::system::Commands;
use bevy::prelude::ChildOf;
use bevy::ecs::system::Query;
use bevy::ecs::system::Res;
use bevy::ecs::message::MessageWriter;
use bevy::time::Time;

use crate::persistence::PersistenceWrite;
use crate::relay;
use crate::relay::PositionPacket;
use crate::render::apply_cheat;
use crate::render::CHEAT_ENTRIES;
use crate::sfx;
use crate::sfx::MarioSfx;
use crate::GameState;

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
    /// Whether a trigger (`MARIO_HEAVY_HIT_BUTTONS`) was held the instant this swing fired. Set
    /// once at fire time, not read live off the gamepad again when the hit actually lands (which
    /// can be a tick or more later, by which point the button may already be released) — a heavy
    /// swing should sound heavy because of how it was thrown, not whether the button still happens
    /// to be down on impact.
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
/// Not yet player-chosen (see `render::PLAYER_FLAIRS`'s own doc comment); every player index maps
/// to this fixed order until a real picker exists.
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
/// reaches full height, unaffected by this constant at all -- only an early release ever takes
/// this path.
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
/// Pressing down in the air is already spoken for, aiming a downward stomp attack instead.
pub const MARIO_CROUCH_SPEED_MULTIPLIER: f32 = 0.45;
pub const MARIO_DUST_EFFECT_DURATION: f32 = 0.35;
pub const MARIO_ATTACK_BUTTON: bevy::input::gamepad::GamepadButton = bevy::input::gamepad::GamepadButton::East;
/// Held during an attack, this swing counts as "heavy" (see `MarioAttackEffect::heavy`) — purely
/// a sound cue difference right now, no extra damage or reach, just a heavier crunch on landing.
/// The analog triggers (`Trigger2`), not the shoulder bumpers (`Trigger`) — what most controllers'
/// own labeling calls LT/RT rather than LB/RB.
pub const MARIO_HEAVY_HIT_BUTTONS: [bevy::input::gamepad::GamepadButton; 2] =
    [bevy::input::gamepad::GamepadButton::LeftTrigger2, bevy::input::gamepad::GamepadButton::RightTrigger2];
/// Either shoulder bumper triggers a dash (see `MARIO_DASH_SPEED_COLUMNS_PER_SEC`) -- `Trigger`, not
/// `Trigger2` (the analog triggers `MARIO_HEAVY_HIT_BUTTONS` already uses), what most controllers'
/// own labeling calls LB/RB rather than LT/RT.
pub const MARIO_DASH_BUTTONS: [bevy::input::gamepad::GamepadButton; 2] =
    [bevy::input::gamepad::GamepadButton::LeftTrigger, bevy::input::gamepad::GamepadButton::RightTrigger];
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
    /// system stays a thin adapter over this and this itself stays independently testable.
    /// `move_input`: -1.0 (full left) to 1.0 (full right). `jump_held`: still down this tick,
    /// separate from the press edge so a held-vs-tapped button can be told apart at all, which is
    /// what pressure-sensitive jumping needs. `body_height`/`body_width`: the play area's current
    /// height/width in rows/columns. `crouch_held`: only actually crouches while `grounded`,
    /// pressing down in the air still just aims a downward stomp. `dash_pressed`: edge-triggered
    /// (like `jump_pressed`/`attack_pressed`), a bumper burst in the left stick's current direction
    /// -- see `MARIO_DASH_SPEED_COLUMNS_PER_SEC`.
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

/// A static surface, `(x0, y0, x1, y1)` in the same 0.0-1.0 fractional space `MarioState::x`/`y`
/// live in. Spawned once at startup and never moved. `passable`: whether physics can ever stop
/// against it at all -- per direct user design intent ("we should be able to define collision per
/// entity"), a platform is no longer always one solid rect; `spawn_platform` now spawns a solid
/// slab paired with a `passable` decorative underside.
///
/// Deliberately, physics itself (`resolve_mario_collisions_and_grounding` below, and everything it
/// calls) never sees this flag at all, only ever a plain `&[(f32, f32, f32, f32)]` rect slice --
/// filtered to `!passable` entities before physics ever runs, so physics doesn't need to know this
/// concept exists. Only the render call site (`main.rs`) needs the full component, since it needs
/// both a rect *and* whether to paint it as light stone (solid) or dark stone (a pass-through
/// underside) -- see `render::mario_body_text`'s own `colliders` param.
#[derive(Component, Clone, Copy)]
pub struct MarioCollider {
    pub rect: (f32, f32, f32, f32),
    pub passable: bool,
}

/// Pushes `mario` back out of any `colliders` it ended up inside of (or swept clean through) after
/// `step` integrated its position for this tick. `prev_x`/`prev_y` are its position *before* that
/// integration (`MarioState::prev_x`/`prev_y`, stamped at the top of `step`).
///
/// Real bug, found live: the original version of this picked a resolution axis purely by *minimum
/// penetration depth* of wherever `mario` ended up this tick, with no notion of which direction it
/// was actually traveling. Against a thin platform (the single static one here is only ~0.03 of
/// the play area tall), a fast fall's single-tick step routinely integrates `y` to somewhere in the
/// *lower* half of that thin box. At that point "least penetration" is the *bottom* face, so the
/// old code shoved the player down and out through the floor of the very platform they were
/// falling onto, reading as "I land in it, then fall through" (exactly the user's own report).
/// Worse, if a fast enough fall's single tick skipped the box's thin `y` range entirely, no
/// overlap was ever detected at all, and the player fell straight through with no resolution. That
/// was the "falls through inconsistently" half of the same report.
///
/// Fixed with a proper swept check, done first and unconditionally on `y` regardless of where the
/// tick's integration left `mario` relative to the box. If the straight line from `(prev_x,
/// prev_y)` to `(mario.x, mario.y)` crosses the platform's top or bottom face while inside its `x`
/// span, that's a real landing/head-bump this tick had, whether or not the final position also
/// happens to still be inside the box. Left/right still resolve from the (now already
/// top/bottom-corrected) final overlap, since lateral speed is never anywhere near enough to tunnel
/// a whole platform width in one tick the way a fall can tunnel its thin height.
///
/// Solid on every face, per the user's own explicit call after trying a one-way (solid-from-above-
/// only) version: a player jumping up into it should bump off the underside exactly like landing
/// on top, not pass through. That one-way detour was chasing the wrong cause anyway. The "avatar
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

/// The seven systems below replace what used to be one large `update_mario_physics` doing all of
/// this inline in a single per-gamepad loop plus three trailing loops over `GameState::mario` — a
/// real, live-flagged problem, not just a style complaint: many small systems with narrow, disjoint
/// concerns are what let Bevy's scheduler reason about a tick at all, and a single system silently
/// sequencing "read input, step physics, trigger sfx, resolve combat, resolve collisions, publish
/// network state" inside its own control flow gives the scheduler nothing to work with — it's a
/// hand-rolled update loop wearing one `System` label, not real ECS decomposition. Split into:
/// `reconcile_gamepad_ownership` (which players exist and are mine to simulate), `handle_cheat_menu_
/// input` (the pause menu), `step_mario_physics` (per-player input decode + physics + jump/attack
/// sfx + respawn/death-effect countdown + new-player registration), `apply_incoming_combat_events`,
/// `resolve_mario_hits` (attack-cone hit detection, local and remote-tracked), `resolve_mario_
/// collisions_and_grounding`, and `publish_local_mario_snapshot`. `main.rs` registers all seven in
/// this exact order via `.chain()`, matching the original single system's own internal ordering
/// exactly — this is a decomposition, not a behavior change. One real ordering subtlety worth
/// stating plainly since it's easy to miss (and exactly the kind of thing that bit us with tonight's
/// render bugs): `resolve_mario_hits` runs on positions from this tick's `step_mario_physics`
/// *before* `resolve_mario_collisions_and_grounding` resolves them against any collider — that was
/// already true in the original single-system code (collision resolution was its own trailing loop,
/// after hit detection, not interleaved per-player with stepping), this split just makes it a
/// visible ordering between two named systems instead of an implicit one between two loops in one
/// function body.
///
/// What this pass deliberately does *not* do: migrate `GameState::mario` off `Arc<RwLock<Vec<...>>>`
/// onto real Bevy `Component`s/`Query`s. Every system below still takes `Res<GameState>` and reaches
/// into the same shared lock, which means Bevy's scheduler still can't see any of these as
/// non-conflicting and still can't actually parallelize them — `.chain()` here is enforcing an order
/// the scheduler has no way to discover on its own, not merely documenting one it already would have
/// picked. That bigger migration (real per-player components, systems with genuinely disjoint
/// queries) is what would unlock real scheduling benefit and is also the actual prerequisite for the
/// "someone else's app plugs in a few systems" reusability goal discussed earlier this session — a
/// second, larger, higher-risk pass, not attempted here.
/// Publishes every gamepad this instance can currently see (read by the ownership-reconciliation
/// task regardless of ownership — it needs to know about every gamepad physically visible, not just
/// the ones this instance currently controls), prunes `GameState::mario` down to just the gamepads
/// this instance is actually allowed to drive right now, and registers a fresh `MarioState` for any
/// owned gamepad that doesn't have one yet. Real, standalone concern from everything downstream:
/// deciding *which players exist this tick*, before physics, the menu, combat, or networking ever
/// touches `mario`.
///
/// The registration half used to live inside `step_mario_physics`'s own per-gamepad match, a real,
/// live-reported architectural mismatch: "player spawn location feels like a startup system, not a
/// physics system." Moved here, which already owns exactly this "which players exist" concern and
/// already runs first in the chain, so a new entry is always in place before `step_mario_physics`
/// looks for it later this same tick.
pub fn reconcile_gamepad_ownership(state: Res<GameState>, gamepads: Query<(Entity, &bevy::input::gamepad::Gamepad)>) {
    *state.visible_gamepad_candidates.write() = gamepads.iter().map(|(_, pad)| gamepad_candidate_id(pad)).collect();

    // No sqld connection, or the first tick or two after connecting before the first sync round
    // lands, means nothing to reconcile against: trust the local view entirely.
    let reconciling = state.persistence.read().is_some();
    let owned = state.gamepad_owned_by_me.read();
    let is_owned = |pad: &bevy::input::gamepad::Gamepad| !reconciling || owned.contains(&gamepad_candidate_id(pad));

    let mut mario = state.mario.write();
    mario.retain(|(entity, ..)| gamepads.get(*entity).is_ok_and(|(_, pad)| is_owned(pad)));

    for (entity, pad) in gamepads.iter() {
        if !is_owned(pad) || mario.iter().any(|(existing, ..)| *existing == entity) {
            continue;
        }
        // On the platform, weighted toward its edges, not the ground center -- per direct user
        // request, so any number of players joining a free-for-all brawl find a reasonable
        // spread-out starting position instead of stacking on the same point.
        let candidate_id = gamepad_candidate_id(pad);
        let (side_random, position_random) = spawn_random_pair(&candidate_id);
        let (x0, y0, x1, _) = platform_slab_rect();
        let x = weighted_edge_x(x0, x1, side_random, position_random);
        mario.push((entity, candidate_id, MarioState { x, y: y0, prev_x: x, prev_y: y0, ..MarioState::default() }));
    }
}

/// The pause menu: opens/closes on a real gamepad Start button (any of this instance's *owned*
/// gamepads can toggle it — the same ownership gate `reconcile_gamepad_ownership` applies, rechecked
/// here since this system doesn't otherwise see that decision), and while open, every owned
/// gamepad's D-pad/South drives its navigation instead of a player. Runs before `step_mario_physics`
/// so this tick's toggle (if any) is already settled by the time physics decides whether to skip a
/// gamepad for "menu is open" — every gamepad sees the same, final open/closed state for this tick,
/// not whatever it happened to be mid-iteration the way the original single loop's own order-
/// dependent read did.
pub fn handle_cheat_menu_input(state: Res<GameState>, gamepads: Query<(Entity, &bevy::input::gamepad::Gamepad)>, time: Res<Time>) {
    let dt = time.delta_secs().clamp(1.0 / 60.0, 0.1);

    // Ghosts freeze too while the pause menu is open: see `GameState::paused_accumulated`.
    if *state.cheat_menu_open.read() {
        *state.paused_accumulated.write() += Duration::from_secs_f32(dt);
    }

    let reconciling = state.persistence.read().is_some();
    let owned = state.gamepad_owned_by_me.read();
    let is_owned = |pad: &bevy::input::gamepad::Gamepad| !reconciling || owned.contains(&gamepad_candidate_id(pad));

    let mut mario = state.mario.write();
    for (_, pad) in gamepads.iter() {
        if !is_owned(pad) {
            continue;
        }

        if pad.just_pressed(bevy::input::gamepad::GamepadButton::Start) {
            let mut open = state.cheat_menu_open.write();
            *open = !*open;
            *state.cheat_menu_selected.write() = 0;
        }

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
        }
    }
}

/// Advances each of this instance's own players by one tick: decodes this gamepad's own stick/
/// d-pad/button state into `MarioState::step`'s own input shape, steps physics for an alive player
/// (with the jump/attack sfx a `step` call can't report on its own — a before/after diff on `jumps_
/// used`/`attack_effect` is the only way to tell "a jump/swing actually fired this tick" from "the
/// button's just being held"), counts a dead-with-lives-left player down to respawn, and registers a
/// newly seen gamepad as a fresh player. Skipped entirely for a gamepad while the pause menu is open
/// (`handle_cheat_menu_input` runs first and settles this tick's open/closed state before this
/// system ever checks it) — matches the *entire* original per-gamepad body being skipped, including
/// a brand new gamepad plugged in mid-pause: it won't be registered until the menu closes, same as
/// before this system was split out.
pub fn step_mario_physics(state: Res<GameState>, gamepads: Query<(Entity, &bevy::input::gamepad::Gamepad)>, time: Res<Time>, mut commands: Commands, sfx: Res<MarioSfx>) {
    let dt = time.delta_secs().clamp(1.0 / 60.0, 0.1);
    let body_height = state.body_rect_seen.read().map(|rect| rect.height as f32).filter(|&height| height > 0.0).unwrap_or(40.0);
    let body_width = state.body_rect_seen.read().map(|rect| rect.width as f32).filter(|&width| width > 0.0).unwrap_or(100.0);

    let reconciling = state.persistence.read().is_some();
    let owned = state.gamepad_owned_by_me.read();
    let is_owned = |pad: &bevy::input::gamepad::Gamepad| !reconciling || owned.contains(&gamepad_candidate_id(pad));

    let mut mario = state.mario.write();

    for (entity, pad) in gamepads.iter() {
        if !is_owned(pad) {
            continue;
        }
        if *state.cheat_menu_open.read() {
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
        let heavy_held = MARIO_HEAVY_HIT_BUTTONS.iter().any(|button| pad.pressed(*button));
        let dash_pressed = MARIO_DASH_BUTTONS.iter().any(|button| pad.just_pressed(*button));

        match mario.iter_mut().find(|(existing, ..)| *existing == entity) {
            Some((_, _, mario_state)) if mario_state.alive => {
                // `step` doesn't report what it did, so a jump/attack sound needs the before/after
                // diff: `jumps_used` only goes up on a jump actually firing (not just the button
                // being held), and `attack_effect` only goes from `None` to `Some` the instant a
                // new swing fires, not on every tick it stays visible.
                let jumps_used_before = mario_state.jumps_used;
                let had_attack_effect = mario_state.attack_effect.is_some();
                mario_state.step(dt, move_input, attack_stick_y, jump_pressed, jump_held, attack_pressed, heavy_held, crouch_held, dash_pressed, body_height, body_width);
                if mario_state.jumps_used > jumps_used_before {
                    sfx::play(&mut commands, &sfx.jump);
                }
                if mario_state.attack_effect.is_some() && !had_attack_effect {
                    sfx::play(&mut commands, &sfx.attack);
                }
            }
            // Dead with lives left: just counts down to a respawn. Dead with no lives left
            // (`respawn_remaining` already `None`) falls through to this arm and does nothing.
            Some((_, _, mario_state)) => {
                if let Some(remaining) = mario_state.respawn_remaining.as_mut() {
                    *remaining -= dt;
                    if *remaining <= 0.0 {
                        mario_state.respawn_remaining = None;
                        mario_state.alive = true;
                        // On the platform, weighted toward its edges -- per direct user request,
                        // same reasoning as the initial-spawn arm below (`weighted_edge_x`'s own
                        // doc comment).
                        let (side_random, position_random) = spawn_random_pair(&gamepad_candidate_id(pad));
                        let (x0, y0, x1, _) = platform_slab_rect();
                        mario_state.x = weighted_edge_x(x0, x1, side_random, position_random);
                        mario_state.y = y0;
                        // Stale otherwise: `prev_x`/`prev_y` still held wherever this player died,
                        // which the very next tick's swept-collision check would sweep a straight
                        // line from -- harmless when every respawn landed at the same fixed point,
                        // but a real glitch risk now that respawns are scattered across the platform.
                        mario_state.prev_x = mario_state.x;
                        mario_state.prev_y = mario_state.y;
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
            // `reconcile_gamepad_ownership` registers a fresh `MarioState` for every owned gamepad
            // before this system ever runs -- a genuine `None` here would mean this gamepad wasn't
            // owned yet this tick (a one-tick ownership-reconciliation race, not a bug in either
            // system), so just skip it rather than spawn a second, possibly-inconsistent entry.
            None => {}
        }
    }
}

/// Combat events a remote attacker landed on one of this instance's own local players (see
/// `relay`'s own doc comment: combat is attacker-authoritative, so this instance is the only one
/// with the standing to apply a death to a player it actually owns). Runs before `resolve_mario_
/// hits` purely for read order; the two never touch the same player in one tick since a dead player
/// can't be found as anyone's local target anyway.
pub fn apply_incoming_combat_events(state: Res<GameState>, mut commands: Commands, sfx: Res<MarioSfx>) {
    let mut mario = state.mario.write();
    for event in std::mem::take(&mut *state.incoming_combat_events.write()) {
        let Some((_, _, target)) = mario.iter_mut().find(|(_, candidate_id, _)| *candidate_id == event.target_candidate_id) else {
            continue; // Not one of this instance's own players; nothing to do.
        };
        apply_local_death(target, &event.target_candidate_id, -event.dx, -event.dy, &state, &mut commands, &sfx);
    }
}

/// Hit detection: any player whose swing is still live and hasn't already landed a hit checks every
/// other player for contact, local first, then remote-tracked ones. A remote target's death can't
/// be applied here directly, this instance has no authority over a player it doesn't own (see this
/// function's own remote branch, and `relay`'s doc comment) — instead it decides the hit and hands
/// it off as a `CombatEvent` for the target's own instance to apply. Runs on positions from this
/// tick's `step_mario_physics`, *before* `resolve_mario_collisions_and_grounding` resolves them
/// against any collider — unchanged from the original single-system code's own ordering (collision
/// resolution was already its own trailing loop there too, not interleaved per-player with
/// stepping), not a new behavior introduced by splitting this out.
pub fn resolve_mario_hits(state: Res<GameState>, mut rumble: MessageWriter<bevy::input::gamepad::GamepadRumbleRequest>, mut commands: Commands, sfx: Res<MarioSfx>) {
    let mut mario = state.mario.write();
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
                attack_cone_hits(attacker_x, attacker_y, effect, target.x, target.y)
            })
            .map(|(target_index, _)| target_index);

        if let Some(target_index) = hit_target_index {
            mario[attacker_index].2.attack_effect.as_mut().unwrap().has_hit = true;
            mario[attacker_index].2.kills += 1;
            play_attack_impact_sfx(&mut commands, &sfx, effect.heavy);

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

            let target_candidate_id = mario[target_index].1.clone();
            apply_local_death(&mut mario[target_index].2, &target_candidate_id, -effect.dx, -effect.dy, &state, &mut commands, &sfx);
            continue;
        }

        // No local target in range. Check remote-tracked players too — the attack cone math is
        // identical, only the position source differs (a `PositionPacket`'s last-known spot
        // instead of a live local `MarioState`, unavoidably a beat stale).
        let remote_target = state.remote_mario.read().iter().find_map(|(candidate_id, packet)| {
            attack_cone_hits(attacker_x, attacker_y, effect, packet.x, packet.y).then(|| candidate_id.clone())
        });
        let Some(target_candidate_id) = remote_target else { continue };

        mario[attacker_index].2.attack_effect.as_mut().unwrap().has_hit = true;
        mario[attacker_index].2.kills += 1;
        play_attack_impact_sfx(&mut commands, &sfx, effect.heavy);
        rumble.write(bevy::input::gamepad::GamepadRumbleRequest::Add {
            gamepad: mario[attacker_index].0,
            intensity: MARIO_HIT_RUMBLE_ATTACKER_INTENSITY,
            duration: MARIO_HIT_RUMBLE_ATTACKER_DURATION,
        });
        state.outgoing_combat_events.write().push(relay::CombatEvent {
            attacker_candidate_id: mario[attacker_index].1.clone(),
            target_candidate_id,
            heavy: effect.heavy,
            dx: effect.dx,
            dy: effect.dy,
        });
    }
}

/// Resolves this tick's movement against every static collider (the swept top/underside checks,
/// see `resolve_mario_collisions`'s own doc comment), then recomputes `grounded`/`touching_wall`
/// from wherever that resolution actually left each player. Landing (on the floor or a platform)
/// refills both jumps. Runs after `resolve_mario_hits` — see that function's own doc comment for why
/// that ordering (hit detection before collision resolution) isn't a new behavior this split
/// introduced.
pub fn resolve_mario_collisions_and_grounding(state: Res<GameState>, colliders: Query<&MarioCollider>) {
    // `!passable` only -- see `MarioCollider`'s own doc comment on why physics never sees the flag
    // itself, only ever a plain, pre-filtered rect slice.
    let collider_rects: Vec<(f32, f32, f32, f32)> = colliders.iter().filter(|collider| !collider.passable).map(|collider| collider.rect).collect();
    for (_, _, mario_state) in state.mario.write().iter_mut() {
        let (prev_x, prev_y) = (mario_state.prev_x, mario_state.prev_y);
        resolve_mario_collisions(mario_state, prev_x, prev_y, &collider_rects);
        mario_state.grounded = is_grounded(mario_state, &collider_rects);
        // Landing, on the real floor or a platform, refills both jumps.
        if mario_state.grounded {
            mario_state.jumps_used = 0;
        }
        mario_state.touching_wall = touching_wall(mario_state, &collider_rects);
    }
}

/// Refreshed every tick regardless of whether anything moved: the relay's own send loop reads this
/// on a fixed interval, and a future authoritative server needs a steady stream, not
/// change-detection. Runs last, after collision resolution, so it publishes each player's actually-
/// resolved position, not the pre-collision one `resolve_mario_hits` used.
pub fn publish_local_mario_snapshot(state: Res<GameState>) {
    *state.local_mario_snapshot.write() = state
        .mario
        .read()
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
            alive: mario_state.alive,
        })
        .collect();
}

/// The attack-cone check `resolve_mario_hits`'s hit detection runs against both a local target's
/// live `MarioState` and a remote target's last-known `PositionPacket` — same geometry either way,
/// only the position source differs, so both call this instead of duplicating it.
fn attack_cone_hits(attacker_x: f32, attacker_y: f32, effect: MarioAttackEffect, target_x: f32, target_y: f32) -> bool {
    let (delta_x, delta_y) = (target_x - attacker_x, target_y - attacker_y);
    let distance = (delta_x * delta_x + delta_y * delta_y).sqrt();
    if distance > MARIO_ATTACK_REACH || distance <= f32::EPSILON {
        return false;
    }
    let alignment = (delta_x / distance) * effect.dx + (delta_y / distance) * effect.dy;
    alignment > MARIO_ATTACK_DIRECTION_COS_THRESHOLD
}

/// The attacker-side "my swing just connected" feedback: shared by a local hit and a hit on a
/// remote-tracked target, since the attacker's own instance always plays this immediately either
/// way, it's only whether the *target*'s death gets applied locally or handed off that differs.
fn play_attack_impact_sfx(commands: &mut Commands, sfx: &MarioSfx, heavy: bool) {
    sfx::play(commands, if heavy { &sfx.heavy_hit } else { &sfx.hit_crunch });
    sfx::play(commands, &sfx.hit_hurt);
}

/// Applies a death to `target`, one of this instance's own locally-owned players — never a remote
/// one, this instance has no authority over those (see `relay`'s own doc comment). Shared by a
/// locally-detected hit and an incoming `CombatEvent` naming a local player as the target, so both
/// paths apply the exact same rules: a full kill, no partial damage, a permanent ghost recorded
/// exactly once. Does nothing if `target` is already dead, since a duplicate incoming combat event
/// (unexpected on a reliable channel, but cheap to guard regardless) shouldn't double-kill.
fn apply_local_death(target: &mut MarioState, target_candidate_id: &str, burst_dx: f32, burst_dy: f32, state: &GameState, commands: &mut Commands, sfx: &MarioSfx) {
    if !target.alive {
        return;
    }

    let (death_x, death_y) = (target.x, target.y);
    target.alive = false;
    sfx::play(commands, &sfx.death);
    target.death_effect = Some(MarioDeathEffect { x: death_x, y: death_y, remaining: MARIO_DEATH_EFFECT_DURATION, burst_dx, burst_dy });
    target.lives = target.lives.saturating_sub(1);
    target.respawn_remaining = if target.lives > 0 { Some(MARIO_RESPAWN_SECONDS) } else { None };

    // Every lost life becomes a permanent floating ghost, regardless of whether this death
    // exhausted this player's lives.
    let (target_name, target_color) = {
        let connected_players = state.connected_players.read();
        connected_players
            .iter()
            .find(|(id, ..)| id == target_candidate_id)
            .map(|(_, name, color)| (name.clone(), *color))
            .unwrap_or_else(|| (target_candidate_id.to_string(), mario_player_color(0)))
    };
    if let Some(sender) = state.persistence_writes.read().as_ref() {
        let _ = sender.send(PersistenceWrite::Defeat { candidate_id: target_candidate_id.to_string(), name: target_name, color: target_color });
    }
}

/// Spawns the single static platform this example plays on, once, at startup. Placed relative to
/// the ground (`MARIO_GROUND_Y`), not a fixed-looking "reasonable" middle height: max jump height
/// is a *fraction of the play area's height* (`MARIO_JUMP_VELOCITY_ROWS_PER_SEC.powi(2) / (2.0 *
/// MARIO_GRAVITY_ROWS_PER_SEC2 * body_height)` rows canceling out), so a fixed fractional platform
/// height only reads as "just under one jump" on the roughly 40-row terminal these values are
/// tuned against. It's computed below from that same reference height, then pulled in slightly
/// (the `0.85` margin) so a full-height jump reliably clears it rather than just barely brushing
/// the underside. On an unusually tall terminal the platform sits proportionally lower and easier
/// to reach, not harder. That's the opposite failure from a fixed offset that's too high to jump to.
const MARIO_PLATFORM_REFERENCE_HEIGHT_ROWS: f32 = 40.0;
const MARIO_PLATFORM_JUMP_HEIGHT_MARGIN: f32 = 0.85;
/// A fixed 60 columns wide, centered, at the reference terminal width below — per direct user
/// aesthetic direction ("a fixed 60 unit width"), replacing the previous "70% of whatever the
/// terminal's actual width happens to be" rule. Still expressed as a fraction (scaling with the
/// real terminal width the same way every other platform dimension in this file does, rather than
/// hardcoding fractional bounds that'd only look like 60 columns at exactly this reference size);
/// the *reference* is now the fixed point instead of the raw fraction. Was 40% (`0.3..0.7`), then a
/// flat 70%.
const MARIO_PLATFORM_REFERENCE_WIDTH_COLUMNS: f32 = 80.0;
const MARIO_PLATFORM_WIDTH_COLUMNS: f32 = 60.0;
const MARIO_PLATFORM_WIDTH_FRACTION: f32 = MARIO_PLATFORM_WIDTH_COLUMNS / MARIO_PLATFORM_REFERENCE_WIDTH_COLUMNS;
/// The gap left between a solid slab's own `y1` and where its paired `passable` underside actually
/// starts, in the same fractional space -- reserved for the slab's own visible "light" body (see
/// `render.rs`'s own doc comment on why that body is painted one row *below* the slab's collision
/// row, never on it). `0.03` comfortably clears one terminal row at the ~40-row reference height
/// (`1/(40-1)` ≈ `0.0256`) with rounding margin, so the light band and the dark underside land on
/// genuinely distinct rows rather than risking collapsing onto the same one at some terminal
/// heights -- this session already hit exactly that class of row-rounding fragility twice before
/// with fractional-position math, so the margin here is deliberate, not arbitrary.
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
const MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION: f32 = 0.03;
/// How tall the passable underside's own dark band is, once it starts (see
/// `MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION` for where it starts). `0.025` reads as roughly one
/// terminal row at the reference height, same reasoning as that constant. Kept in sync by value
/// (not by import -- see `MarioCollider`'s own doc comment on why this file has no dependency on
/// `mario-core`) with that crate's own `state::MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION`, which
/// `mario-wasm` uses directly.
const MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION: f32 = 0.025;

/// Marks the parent entity of a platform's collider set (currently just the one static platform's
/// slab + underside). Per direct user request, purely so the two colliders are real Bevy children
/// of a common parent, not two independent top-level entities that happen to share coordinates --
/// queryable and despawnable as a unit (Bevy's own hierarchy already recursively despawns children
/// when a parent despawns), and a real anchor for a future "move this whole platform" operation to
/// hang off of. Not wired up yet: colliders store plain fractional `rect`s, not `Transform`s, so
/// this parenting is organizational only for now -- it doesn't make moving the parent's (nonexistent)
/// `Transform` reposition the children automatically. A real move operation still needs to update
/// both `rect`s directly; the parent-child relationship is what makes finding "both colliders that
/// belong to this platform" together a real query instead of an assumption about spawn order.
#[derive(Component)]
pub struct MarioPlatform;

/// The solid slab's own `(x0, y0, x1, y1)`, pure and parameter-free -- every input is a file-level
/// constant, so this needs no `Commands`/ECS access at all. Extracted so `spawn_platform` and the
/// player-spawn-position logic (`step_mario_physics`'s own `None =>` arm, and the death-timer
/// respawn branch) compute the *exact* same rect without duplicating the arithmetic, which the
/// respawn/spawn positions need to place a player relative to the platform.
fn platform_slab_rect() -> (f32, f32, f32, f32) {
    let jump_apex_rows =
        (MARIO_JUMP_VELOCITY_ROWS_PER_SEC * MARIO_JUMP_VELOCITY_ROWS_PER_SEC) / (2.0 * MARIO_GRAVITY_ROWS_PER_SEC2);
    let height_above_ground =
        (jump_apex_rows / MARIO_PLATFORM_REFERENCE_HEIGHT_ROWS) * MARIO_PLATFORM_JUMP_HEIGHT_MARGIN;
    let y1 = MARIO_GROUND_Y - height_above_ground;
    let y0 = y1 - 0.02;
    let x0 = (1.0 - MARIO_PLATFORM_WIDTH_FRACTION) / 2.0;
    let x1 = x0 + MARIO_PLATFORM_WIDTH_FRACTION;
    (x0, y0, x1, y1)
}

pub fn spawn_platform(mut commands: Commands) {
    let (x0, y0, x1, y1) = platform_slab_rect();
    let underside_y0 = y1 + MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION;

    let platform = commands.spawn(MarioPlatform).id();
    commands.spawn((MarioCollider { rect: (x0, y0, x1, y1), passable: false }, ChildOf(platform)));
    commands.spawn((
        MarioCollider { rect: (x0, underside_y0, x1, underside_y0 + MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION), passable: true },
        ChildOf(platform),
    ));
}

/// Two independent, cheap pseudo-random values in `[0.0, 1.0)` for `state::weighted_edge_x` --
/// native has no existing RNG dependency in this workspace worth adding just for this (`mario-wasm`
/// uses the browser's own `js_sys::Math::random()` instead, its own real entropy source), so this
/// is a small xorshift64 seeded from real wall-clock time mixed with `salt` (each call site passes
/// something that varies per spawn -- the joining player's own candidate id -- so two players
/// joining within the same clock tick still land on different seeds, not identical ones). Not
/// cryptographic, doesn't need to be: this only ever decides where a player's sprite starts, the
/// same "cheap hash stands in for real randomness" tradeoff `render.rs`'s own stone texture makes.
fn spawn_random_pair(salt: &str) -> (f32, f32) {
    use std::hash::Hash;
    use std::hash::Hasher;
    let nanos = std::time::SystemTime::now().duration_since(std::time::UNIX_EPOCH).map(|d| d.as_nanos() as u64).unwrap_or(0);
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    salt.hash(&mut hasher);
    let mut seed = nanos ^ hasher.finish() ^ 0x9E3779B97F4A7C15;
    let mut next_u64 = || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };
    let a = (next_u64() >> 11) as f32 / (1u64 << 53) as f32;
    let b = (next_u64() >> 11) as f32 / (1u64 << 53) as f32;
    (a, b)
}

/// Where a newly-spawned (or respawned) player lands along the platform's own width, per direct
/// user request for a free-for-all brawl: "load at a semi-random location weighted heavier towards
/// the edge, with an even split. The effect is that any number of players could join, and they
/// would find a reasonable starting position." `side_random < 0.5` picks the left edge, otherwise
/// the right -- an even split, not weighted either direction. `position_random` is squared before
/// use, biasing it toward `0.0`, then scaled across just the near half of the platform's width
/// (edge to center) -- so a spawn is always at least as close to its own edge as to the center.
/// Kept in sync by value (not by import -- see `MarioCollider`'s own doc comment on why this file
/// has no dependency on `mario-core`) with that crate's own `state::weighted_edge_x`.
fn weighted_edge_x(platform_x0: f32, platform_x1: f32, side_random: f32, position_random: f32) -> f32 {
    let half_width = (platform_x1 - platform_x0) / 2.0;
    let edge_bias = position_random.clamp(0.0, 1.0).powi(2) * half_width;
    if side_random < 0.5 { platform_x0 + edge_bias } else { platform_x1 - edge_bias }
}

#[cfg(test)]
mod tests {
    use super::*;

    const PLATFORM: (f32, f32, f32, f32) = (0.3, 0.91, 0.7, 0.93);

    #[test]
    fn fast_fall_lands_on_a_thin_platform() {
        let mut mario = MarioState { x: 0.5, prev_x: 0.5, y: 0.905, prev_y: 0.905, vy: 1.0, ..MarioState::default() };
        // One tick's worth of a fast fall, integrated exactly the way `step` does, deliberately
        // large enough to jump clean over the platform's own 0.02-tall span in a single step.
        // This is the exact scenario the old min-penetration resolver got wrong (see
        // `resolve_mario_collisions`'s own doc comment).
        mario.y += mario.vy * 0.1;
        assert!(mario.y > PLATFORM.3, "test setup should overshoot clean past the platform, got y={}", mario.y);

        let (prev_x, prev_y) = (mario.prev_x, mario.prev_y);
        resolve_mario_collisions(&mut mario, prev_x, prev_y, &[PLATFORM]);

        assert_eq!(mario.y, PLATFORM.1, "should land exactly on the platform's top surface, not tunnel through it");
        assert_eq!(mario.vy, 0.0);
    }

    #[test]
    fn landing_resolves_to_the_platforms_top() {
        // Ends the tick already inside the thin box, closer to its bottom face than its top.
        // This is exactly the case the old "least penetration" logic got backwards.
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
