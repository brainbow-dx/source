//! Permanent floating markers for every lost life, ever. Each ghost drifts on a slow, independent
//! wander derived once from its own identity so every instance renders the same ghost at the same
//! spot with no coordination beyond the ordinary persistence sync that already loads them.

/// One permanent record of a lost life.
#[derive(Clone)]
pub struct GhostEntry {
    pub candidate_id: String,
    pub name: String,
    pub color: (u8, u8, u8),
    pub drift: GhostDrift,
}

/// Independent per-ghost drift parameters. A shared phase with one common frequency reads as a
/// train of ghosts following each other rather than each wandering on its own, since they'd only
/// differ by a fixed time offset into the same curve. Giving each ghost its own frequency on both
/// axes, not just its own phase, is what actually breaks that resemblance.
#[derive(Clone, Copy)]
pub struct GhostDrift {
    pub x_phase: f32,
    pub y_phase: f32,
    pub x_freq: f32,
    pub y_freq: f32,
    /// A faster, smaller secondary flutter layered on the slow primary drift, for a floatier feel
    /// than a single clean sine sweep gives.
    pub wobble_phase: f32,
    /// Drives this ghost's own swoop timing and fixed swoop direction. See `mario_ghost_swoop`.
    pub chaos_phase: f32,
    /// Drives how visible this ghost currently is. See `mario_ghost_flicker`.
    pub flicker_phase: f32,
}

/// A small twinkling orb — always the same tiny dot glyph now (the user's own direction: ghosts
/// were "too prominent" and "in the way" as a 3-size glyph that grows to a full `•` at its
/// brightest). Kept as a single character rather than inlining `'·'` at both call sites so a
/// future "shrink further at the deep dim floor" tweak still has one place to add a second glyph.
pub const MARIO_GHOST_GLYPH_DIM: char = '·';
/// How dim a ghost renders relative to its own color at its brightest. The actual per-frame
/// brightness is this times the flicker function's own 0.0-1.0 result, never this alone. Lowered
/// per the user's own "more faded" direction — ghosts are a background memorial, not something
/// meant to visually compete with a live player for attention.
pub const MARIO_GHOST_DIM_FRACTION: f64 = 0.32;
/// Ghosts drift within this vertical band, a fraction of the play area's height, `0.0` the very
/// top. Extended further down the screen per the user's own "dither down the screen a bit more"
/// direction — confining every ghost to a thin strip near the top read as clutter right where nothing
/// else happens; spreading the same ghosts over most of the play area reads as much less crowded even
/// though the count hasn't changed.
pub const MARIO_GHOST_BAND_TOP: f32 = 0.02;
pub const MARIO_GHOST_BAND_HEIGHT: f32 = 0.6;
/// How far a ghost's horizontal wander reaches from center (0.5 plus or minus this).
pub const MARIO_GHOST_X_SPREAD: f32 = 0.46;
/// Baseline angular speed of a ghost's wander. Each ghost's actual frequency is this, jittered
/// independently per axis per ghost, never used directly on its own. Halved per the user's own
/// "slower" direction.
pub const MARIO_GHOST_DRIFT_BASE_FREQ: f32 = 0.009;
/// The secondary flutter's own amplitude and frequency: a slow, gentle bob, not a fast buzz.
pub const MARIO_GHOST_WOBBLE_AMPLITUDE: f32 = 0.012;
pub const MARIO_GHOST_WOBBLE_FREQ: f32 = 0.08;
/// An occasional quick directional dash layered on the slow drift, so ghosts mostly wander gently
/// with a rare, distinct dart rather than a continuous fast jitter. `_BASE`/`_JITTER` give each
/// ghost its own average interval between swoops so they never swoop in lockstep. `_WINDOW` is how
/// much of that interval the dash occupies, eased in and back out rather than snapped. `_DISTANCE`
/// is how far it travels. Interval raised and distance shortened per "slower" — swoops were the
/// most eye-catching, least ghost-like part of the old motion.
pub const MARIO_GHOST_SWOOP_INTERVAL_BASE: f32 = 26.0;
pub const MARIO_GHOST_SWOOP_INTERVAL_JITTER: f32 = 20.0;
pub const MARIO_GHOST_SWOOP_WINDOW: f32 = 0.18;
pub const MARIO_GHOST_SWOOP_DISTANCE: f32 = 0.06;
/// The ambient twinkle's own frequency. `_FLOOR` is how far the ambient twinkle alone ever dims on
/// its own, deliberately shallow since the dramatic dimming is `MARIO_GHOST_DEEP_DIM_FLOOR` below.
pub const MARIO_GHOST_FLICKER_FREQ: f32 = 0.45;
pub const MARIO_GHOST_FLICKER_FLOOR: f64 = 0.75;
/// An occasional, much deeper and longer dip toward near-invisible and back, layered on top of the
/// ambient twinkle rather than the same thing sped up. `_PERIOD_BASE`/`_PERIOD_JITTER` give each
/// ghost its own average interval between dips. `_WINDOW` is how much of that interval is spent
/// dipping, eased via a half sine. `_FLOOR` is how dim the dip actually goes.
pub const MARIO_GHOST_DEEP_DIM_PERIOD_BASE: f32 = 7.0;
pub const MARIO_GHOST_DEEP_DIM_PERIOD_JITTER: f32 = 6.0;
pub const MARIO_GHOST_DEEP_DIM_WINDOW: f32 = 0.4;
pub const MARIO_GHOST_DEEP_DIM_FLOOR: f64 = 0.12;
/// The most recent ghosts actually rendered, oldest dropped first. Every ghost is still persisted
/// in full regardless: this only bounds the display, so a long session doesn't crowd the screen
/// with hundreds of them.
pub const MARIO_GHOST_RENDER_LIMIT: usize = 40;

/// Deterministic, fixed per-ghost drift parameters, derived once from `candidate_id` plus
/// `defeated_at`, both permanent facts about a specific lost life, rather than real randomness.
/// Every instance computes the same drift for the same ghost with no coordination needed. Seven
/// independent hashes, not one shared value reused with fixed multipliers, since deriving later
/// phases from the first with a fixed ratio would leave every ghost's curve a fixed offset from
/// the others rather than genuinely independent.
pub fn mario_ghost_drift(candidate_id: &str, defeated_at: i64) -> GhostDrift {
    let hashed = |salt: u8| -> f32 {
        use std::hash::Hash;
        use std::hash::Hasher;
        let mut hasher = std::collections::hash_map::DefaultHasher::new();
        candidate_id.hash(&mut hasher);
        defeated_at.hash(&mut hasher);
        salt.hash(&mut hasher);
        (hasher.finish() % 100_000) as f32 / 100_000.0
    };

    const TAU: f32 = std::f32::consts::TAU;
    GhostDrift {
        x_phase: hashed(0) * TAU,
        y_phase: hashed(1) * TAU,
        // Each ghost's own frequency spans roughly 0.4x to 1.6x the baseline, wide enough that two
        // ghosts' curves visibly diverge instead of staying in lockstep.
        x_freq: MARIO_GHOST_DRIFT_BASE_FREQ * (0.4 + hashed(2) * 1.2),
        y_freq: MARIO_GHOST_DRIFT_BASE_FREQ * (0.4 + hashed(3) * 1.2),
        wobble_phase: hashed(4) * TAU,
        chaos_phase: hashed(5) * TAU,
        flicker_phase: hashed(6) * TAU,
    }
}

/// A ghost's current position: a pure function of `elapsed_seconds` and the ghost's own fixed
/// `drift`, not simulated state updated per tick, so every instance renders every ghost at the
/// identical spot without syncing anything. Different per-ghost x/y angular frequencies trace a
/// Lissajous curve unique to each ghost, plus a faster secondary wobble and an occasional swoop.
pub fn mario_ghost_position(elapsed_seconds: f32, drift: GhostDrift) -> (f32, f32) {
    let x_base = 0.5 + MARIO_GHOST_X_SPREAD * (elapsed_seconds * drift.x_freq + drift.x_phase).sin();
    let y_base = MARIO_GHOST_BAND_TOP + MARIO_GHOST_BAND_HEIGHT * (0.5 + 0.5 * (elapsed_seconds * drift.y_freq + drift.y_phase).cos());
    let wobble = MARIO_GHOST_WOBBLE_AMPLITUDE * (elapsed_seconds * MARIO_GHOST_WOBBLE_FREQ + drift.wobble_phase).sin();
    let (swoop_x, swoop_y) = mario_ghost_swoop(elapsed_seconds, drift);
    ((x_base + swoop_x).clamp(0.0, 1.0), (y_base + wobble + swoop_y).clamp(0.0, 1.0))
}

/// A rare, quick directional dash. `elapsed_seconds` divided by this ghost's own swoop interval
/// gives a repeating 0.0-1.0 cycle position. Only the first `MARIO_GHOST_SWOOP_WINDOW` fraction of
/// each cycle actually swoops, eased in and back out via a half sine, in a fixed direction also
/// derived from `chaos_phase`, so each ghost swoops on its own schedule and its own heading.
pub fn mario_ghost_swoop(elapsed_seconds: f32, drift: GhostDrift) -> (f32, f32) {
    let interval = MARIO_GHOST_SWOOP_INTERVAL_BASE + (drift.chaos_phase / std::f32::consts::TAU) * MARIO_GHOST_SWOOP_INTERVAL_JITTER;
    let cycle_position = (elapsed_seconds / interval).fract();
    if cycle_position >= MARIO_GHOST_SWOOP_WINDOW {
        return (0.0, 0.0);
    }

    let intensity = ((cycle_position / MARIO_GHOST_SWOOP_WINDOW) * std::f32::consts::PI).sin();
    let angle = drift.chaos_phase;
    (angle.cos() * MARIO_GHOST_SWOOP_DISTANCE * intensity, angle.sin() * MARIO_GHOST_SWOOP_DISTANCE * intensity)
}

/// How visible a ghost currently is, `0.0` (faded almost entirely dim) to `1.0` (as bright as
/// `MARIO_GHOST_DIM_FRACTION` ever allows). Two layered waves: a slow ambient twinkle that only
/// ever softens the orb a little, plus an occasional, much deeper dip on its own longer, slower
/// period, so the two read as genuinely separate behaviors rather than one flicker at two speeds.
pub fn mario_ghost_flicker(elapsed_seconds: f32, phase: f32) -> f64 {
    let ambient = ((elapsed_seconds * MARIO_GHOST_FLICKER_FREQ + phase).sin() as f64) * 0.5 + 0.5;
    let ambient = MARIO_GHOST_FLICKER_FLOOR + (1.0 - MARIO_GHOST_FLICKER_FLOOR) * ambient;

    let deep_dim_period = MARIO_GHOST_DEEP_DIM_PERIOD_BASE + (phase / std::f32::consts::TAU) * MARIO_GHOST_DEEP_DIM_PERIOD_JITTER;
    let deep_dim_cycle = (elapsed_seconds / deep_dim_period).rem_euclid(1.0);
    let deep_dim = if deep_dim_cycle < MARIO_GHOST_DEEP_DIM_WINDOW {
        let eased = ((deep_dim_cycle / MARIO_GHOST_DEEP_DIM_WINDOW) * std::f32::consts::PI).sin() as f64;
        1.0 - eased * (1.0 - MARIO_GHOST_DEEP_DIM_FLOOR)
    } else {
        1.0
    };

    (ambient * deep_dim).clamp(MARIO_GHOST_DEEP_DIM_FLOOR, 1.0)
}

/// Which orb glyph a ghost renders as this frame. Always the same tiny dot now — see
/// `MARIO_GHOST_GLYPH_DIM`'s own doc comment — the flicker value still drives how dim it is
/// (`render.rs`'s own color blend), just not its size/glyph anymore.
pub fn mario_ghost_glyph(_flicker: f64) -> char {
    MARIO_GHOST_GLYPH_DIM
}
