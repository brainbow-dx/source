//! Small, generic animation math — no `Scaffold`/surface involvement, just numbers in and out.
//! Extracted from `apps/anvil`'s `main.rs`: neither function was ever Anvil-specific, they'd
//! just accumulated in the app instead of the library they actually belong in. Any surface
//! composing a pulsing indicator or interpolating a color over time can reuse these directly.

use std::time::Duration;

/// A smooth 0..1 "breathing" value from a sine wave over `period`, rather than something that
/// snaps between fixed states — reads as a calm, quietly-alive indicator instead of a blink.
pub fn breathe(elapsed: Duration, period: Duration) -> f64 {
    let phase = (elapsed.as_millis() % period.as_millis().max(1)) as f64 / period.as_millis() as f64;
    (phase * std::f64::consts::TAU).sin() * 0.5 + 0.5
}

/// Linearly interpolates between two `(r, g, b)` colors at `t` (0.0 = `a`, 1.0 = `b`).
pub fn lerp_color(a: (u8, u8, u8), b: (u8, u8, u8), t: f64) -> (u8, u8, u8) {
    let lerp = |x: u8, y: u8| (x as f64 + (y as f64 - x as f64) * t).round() as u8;
    (lerp(a.0, b.0), lerp(a.1, b.1), lerp(a.2, b.2))
}
