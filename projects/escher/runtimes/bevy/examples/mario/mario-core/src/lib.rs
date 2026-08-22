//! The Bevy-free heart of the `mario` example's game feel: player movement/collision/combat state
//! and the terminal-grid text renderer, shared between the native Bevy build
//! (`runtimes/bevy/examples/mario`) and the browser/wasm build (`mario-wasm`, this crate's
//! sibling), so "how the game actually plays" lives in exactly one place.
//!
//! Deliberately narrow: this is `MarioState::step` (pure physics/combat simulation, no gamepad
//! reading, no networking, no persistence) plus `render::mario_body_text` (pure grid-to-string
//! rendering, no terminal I/O). Everything Bevy-specific in the native example — gamepad input,
//! `Commands`/`Query`/`Res`, WebRTC relay, sqld persistence, sound — stays in
//! `runtimes/bevy/examples/mario` itself, unduplicated here.
//!
//! **A real API boundary worth naming explicitly, per direct user question**: `state.rs`
//! (`MarioState::step`, collision, `is_grounded`) is genuinely backend-agnostic — plain fractional
//! `0.0..1.0` coordinates, no notion of a cell, row, or pixel anywhere. `render.rs` is not that: it
//! is specifically a **terminal-grid text renderer** (quantizes those same fractional coordinates
//! into discrete rows/columns and produces an ANSI string), one particular backend among possibly
//! several, not "the" renderer for this game's state. The native example's own Bevy scene window
//! (`runtimes/bevy/examples/mario/scene.rs`) is proof this distinction already matters in practice:
//! it renders the same `MarioState` in continuous world-space (real `Transform`/`Vec3`, no
//! row-quantization at all) and, tellingly, never needed `render.rs`'s own row-shifting fix for
//! "a standing player's sprite overlapping the platform's own top-texture cell" — that bug is
//! inherent to discrete-cell rendering and doesn't exist in continuous space. A future second
//! renderer (a real 2D wgpu/vulkan backend, say) should consume `state.rs` directly and never touch
//! `render.rs`'s grid-specific helpers (`mario_body_text`, `platform_visual_rows`, `to_cell`) at
//! all — those names read more generic than they are today; tightening that (a clearer module name,
//! or a trait if a second grid-shaped backend ever actually needs to share this file) is real,
//! worthwhile follow-up, just not attempted under tonight's playtest deadline.
//!
//! A NOTE ON DUPLICATION, HONESTLY STATED: this crate's contents were copied out of the native
//! example's `physics.rs`/`render.rs`/`ghosts.rs`, not moved — the native files still define their
//! own copies of `MarioState`/`step`/collision helpers/`mario_body_text` rather than importing
//! from here. That's a deliberate, disclosed tradeoff: those files are load-bearing for tonight's
//! LAN playtest, and mechanically rewiring them to re-export from a brand-new crate under time
//! pressure was judged riskier than a small, explicit duplication. This crate is the correct
//! long-term home for this logic; the native example's own copies should be deleted and replaced
//! with `pub use mario_core::*` once there's room to verify that rewiring carefully, not under a
//! playtest deadline. Until then, a change to player movement/combat feel or the grid renderer
//! needs to land in both places by hand — flagged here so that's not a surprise later.

pub mod ghosts;
pub mod render;
pub mod state;

pub use ghosts::GhostDrift;
pub use ghosts::GhostEntry;
pub use render::mario_body_text;
pub use render::mario_player_flair;
pub use state::MarioAttackEffect;
pub use state::MarioDeathEffect;
pub use state::MarioDustEffect;
pub use state::MarioState;
pub use state::mario_player_color;
