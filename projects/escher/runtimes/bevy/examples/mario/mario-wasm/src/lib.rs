//! Browser build of the mario example's solo game feel, driven from JS via `wasm-bindgen` instead
//! of Bevy/`gilrs`/crossterm. Pairs with `web/index.html` (xterm.js harness, checked into this
//! crate's own `web/` directory) — see this crate's sibling `README.md` for how to build and run
//! it, and exactly what's cut from the native example (ghosts, sound, the pause menu, and combat
//! sync specifically — see this module's own networking section below for what IS wired up).
//!
//! Deliberately thin: all real game logic (`MarioState::step`, collision, the ANSI grid renderer)
//! lives in `mario-core`; this crate is the JS-facing adapter (keyboard/gamepad state in, one ANSI
//! frame string out, once per animation frame) plus the LAN-multiplayer wiring described below.
//!
//! ## Networking
//!
//! Joins the same `atlas-relay` room a native `mario --host`/`--connect` session uses, via that
//! crate's own `client_wasm` module (`web-sys`'s browser-native `RtcPeerConnection`, since
//! `webrtc-rs` — what the native `relay.rs` uses — doesn't target wasm32). `PositionPacket` here is
//! a deliberate,
//! disclosed duplicate of `relay.rs`'s own struct of the same name: identical fields/serde shape so
//! JSON round-trips between a browser peer and a native one, but there's no way to depend on it
//! directly since `relay.rs` lives inside the native `mario` example binary, not a library crate.
//! Position sync only — a remote peer's motion is sent/received and rendered as an extra sprite,
//! the same way the native `main.rs`'s own remote-render path works. **Combat sync is not wired up
//! this pass**: the combat data channel is opened (so a native peer's own `on_data_channel` sees a
//! recognized label, matching its channel set) but this crate never sends or applies a
//! `CombatEvent` — a browser player can't hit or be hit yet. Real, scoped future work, not a
//! technical blocker: the attack-cone-hits logic (`physics.rs`'s `attack_cone_hits`) would need
//! porting into `mario-core` the same way movement/collision already were.

use std::collections::HashMap;

use mario_core::MarioState;
use mario_core::mario_body_text;
use mario_core::mario_player_color;
use mario_core::state;
use wasm_bindgen::JsCast;
use wasm_bindgen::JsValue;
use wasm_bindgen::prelude::wasm_bindgen;

/// Same fractional-height-above-ground derivation as the native example's own
/// `physics::spawn_platform` (`MARIO_JUMP_VELOCITY_ROWS_PER_SEC.powi(2) / (2.0 *
/// MARIO_GRAVITY_ROWS_PER_SEC2)` rows canceling out, pulled in by an 0.85 margin so a full-height
/// jump reliably clears it). **Real, live-verified bug this replaced**: an earlier version of this
/// constant borrowed `physics.rs`'s `#[cfg(test)] mod tests`-only `PLATFORM` rect (`(0.3, 0.91,
/// 0.7, 0.93)`) instead — that rect exists purely to give collision *unit tests* a fixed target to
/// overshoot into from an artificially placed `mario.y`, never vetted as reachable from a real
/// spawn. At `y=0.91`, it sat as a low ceiling directly above the default spawn column (`x=0.5`,
/// inside the rect's `[0.3, 0.7]` span), so a real jump from the ground bonked its underside almost
/// immediately and never got anywhere near the platform's top — reproduced deterministically via a
/// headless `Game::tick` loop: the player's rendered row barely moved for the whole held-jump
/// window. Deriving the real reachable height here, the same way the native game does, fixes it.
fn platform_rect() -> (f32, f32, f32, f32) {
    const REFERENCE_HEIGHT_ROWS: f32 = 40.0;
    const JUMP_HEIGHT_MARGIN: f32 = 0.85;
    // A fixed 60 columns wide at the reference terminal width below, centered -- matches the native
    // example's own `physics::MARIO_PLATFORM_WIDTH_FRACTION`, per direct user aesthetic direction
    // ("a fixed 60 unit width"), replacing the previous flat-70%-of-actual-width rule.
    const REFERENCE_WIDTH_COLUMNS: f32 = 80.0;
    const WIDTH_COLUMNS: f32 = 60.0;
    const WIDTH_FRACTION: f32 = WIDTH_COLUMNS / REFERENCE_WIDTH_COLUMNS;
    let jump_apex_rows =
        (state::MARIO_JUMP_VELOCITY_ROWS_PER_SEC * state::MARIO_JUMP_VELOCITY_ROWS_PER_SEC) / (2.0 * state::MARIO_GRAVITY_ROWS_PER_SEC2);
    let height_above_ground = (jump_apex_rows / REFERENCE_HEIGHT_ROWS) * JUMP_HEIGHT_MARGIN;
    let y1 = state::MARIO_GROUND_Y - height_above_ground;
    let y0 = y1 - 0.02;
    let x0 = (1.0 - WIDTH_FRACTION) / 2.0;
    let x1 = x0 + WIDTH_FRACTION;
    (x0, y0, x1, y1)
}

/// Deliberate duplicate of `relay.rs`'s struct of the same name — see this module's own doc
/// comment's "Networking" section for why.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
struct PositionPacket {
    candidate_id: String,
    seq: u32,
    x: f32,
    y: f32,
    vx: f32,
    vy: f32,
    grounded: bool,
    jumps_used: u8,
    alive: bool,
}

const POSITION_CHANNEL_LABEL: &str = "mario-sync";
const COMBAT_CHANNEL_LABEL: &str = "mario-combat";
/// Matches `relay.rs`'s own `SEND_INTERVAL` — about 30Hz, the same cadence a native peer sends at,
/// so a browser peer looks like an ordinary participant on the wire, not a special case.
const SEND_INTERVAL_SECS: f32 = 0.033;

/// One browser tab's whole game: a single local player, driven by keyboard and/or gamepad state
/// (both work simultaneously — see `poll_gamepad`), optionally synced with native peers over the
/// same relay room a `--host`/`--connect` session uses (see this module's own doc comment).
#[wasm_bindgen]
pub struct Game {
    mario: MarioState,
    platform: (f32, f32, f32, f32),
    candidate_id: String,
    keys_down: std::collections::HashSet<String>,
    jump_was_held: bool,
    attack_was_held: bool,
    dash_was_held: bool,
    relay: Option<atlas_relay::client_wasm::RelayClient>,
    remote: HashMap<String, PositionPacket>,
    send_accum: f32,
    seq: u32,
}

#[wasm_bindgen]
impl Game {
    #[wasm_bindgen(constructor)]
    pub fn new() -> Game {
        let candidate_id = format!("browser-{:x}", (js_sys::Math::random() * 1e15) as u64);
        let platform = platform_rect();
        // On the platform, weighted toward its edges -- per direct user request, so any number of
        // players joining a free-for-all brawl find a reasonable spread-out starting position
        // instead of stacking on the ground center. `js_sys::Math::random()` is this crate's own
        // real entropy source (already used for `candidate_id` above); the actual placement math
        // is `mario-core`'s own `state::weighted_edge_x`, shared rather than duplicated since this
        // crate already depends on that crate directly.
        let (side_random, position_random) = (js_sys::Math::random() as f32, js_sys::Math::random() as f32);
        let x = state::weighted_edge_x(platform.0, platform.2, side_random, position_random);
        let y = platform.1;
        Game {
            mario: MarioState { x, y, prev_x: x, prev_y: y, ..MarioState::default() },
            platform,
            candidate_id,
            keys_down: std::collections::HashSet::new(),
            jump_was_held: false,
            attack_was_held: false,
            dash_was_held: false,
            relay: None,
            remote: HashMap::new(),
            send_accum: 0.0,
            seq: 0,
        }
    }

    /// Joins `room` on the `atlas-relay` signaling server at `ws_url` (e.g. `ws://127.0.0.1:9200/
    /// ws` for a host running on this same machine, or that host's LAN IP for a `--connect`-style
    /// join) — the exact same relay a native `mario --host`/`--connect` session uses, so a browser
    /// tab and a native peer see each other. Safe to not call at all: an unconnected `Game` just
    /// keeps playing solo, matching the native example's own "no relay reachable, local play keeps
    /// working" posture.
    pub fn connect(&mut self, ws_url: String, room: String) -> Result<(), wasm_bindgen::JsValue> {
        let channels = serde_json::json!([
            {"label": POSITION_CHANNEL_LABEL, "ordered": false, "maxRetransmits": 0},
            {"label": COMBAT_CHANNEL_LABEL, "ordered": true, "maxRetransmits": null},
        ]);
        self.relay = Some(atlas_relay::client_wasm::RelayClient::new(ws_url, room, channels.to_string())?);
        Ok(())
    }

    /// How many peers this tab currently has an open data-channel connection to — `0` before
    /// `connect` is called, while the handshake is still in flight, or if it's never reachable.
    #[wasm_bindgen(js_name = connectedPeerCount)]
    pub fn connected_peer_count(&self) -> usize {
        self.relay.as_ref().map(|relay| relay.connected_peer_count()).unwrap_or(0)
    }

    /// Called from JS on every `keydown`/`keyup`, keyed by the browser's own `KeyboardEvent.code`
    /// (`"ArrowLeft"`, `"Space"`, ...) — a held-vs-tapped distinction (needed for pressure-sensitive
    /// jumping, see `MarioState::step`'s own doc comment) only works with real press/release state,
    /// not `onData`'s one-shot text events, which is why this reads `keydown`/`keyup` directly
    /// rather than going through xterm.js's own input handling at all.
    #[wasm_bindgen(js_name = setKey)]
    pub fn set_key(&mut self, code: String, pressed: bool) {
        if pressed {
            self.keys_down.insert(code);
        } else {
            self.keys_down.remove(&code);
        }
    }

    /// Advances the simulation by `dt` seconds and returns one ANSI-colored frame, `height` lines
    /// joined by `\n`, ready to hand straight to xterm.js's `write()`. `width`/`height` are the
    /// terminal's current size in columns/rows (JS reads this off the live `Terminal` instance each
    /// frame, since xterm.js can resize). Reads live gamepad state directly (no JS-side wiring
    /// needed — see `poll_gamepad`) in addition to whatever `setKey` has recorded, so keyboard and
    /// gamepad both work at once.
    pub fn tick(&mut self, dt: f32, width: u16, height: u16) -> String {
        let gamepad = poll_gamepad();

        let mut move_input = self.axis("ArrowLeft", "ArrowRight");
        if move_input == 0.0 {
            move_input = gamepad.stick_x;
        }
        // Keyboard has no analog stick to aim an attack or dash with, so it only ever aims along
        // `facing` (`stick_unit_direction`'s own neutral-stick fallback) -- a real gamepad's stick
        // is the only source of vertical aim here.
        let stick_y = gamepad.stick_y;
        let jump_held = self.any_held(&["ArrowUp", "Space", "KeyW"]) || gamepad.south;
        let jump_pressed = jump_held && !self.jump_was_held;
        let attack_held = self.any_held(&["KeyX", "KeyJ"]) || gamepad.east;
        let attack_pressed = attack_held && !self.attack_was_held;
        let crouch_held = self.any_held(&["ArrowDown", "KeyS"]) || gamepad.down;
        let dash_held = gamepad.left_bumper || gamepad.right_bumper;
        let dash_pressed = dash_held && !self.dash_was_held;

        // Solid-only, for physics -- matches how the native example filters colliders down before
        // physics ever sees them (see `state::MarioCollider`'s own doc comment).
        let colliders = [self.platform];
        self.mario.step(dt, move_input, stick_y, jump_pressed, jump_held, attack_pressed, false, crouch_held, dash_pressed, height.max(1) as f32, width.max(1) as f32);
        let (prev_x, prev_y) = (self.mario.prev_x, self.mario.prev_y);
        state::resolve_mario_collisions(&mut self.mario, prev_x, prev_y, &colliders);
        self.mario.grounded = state::is_grounded(&self.mario, &colliders);
        if self.mario.grounded {
            self.mario.jumps_used = 0;
        }
        self.mario.touching_wall = state::touching_wall(&self.mario, &colliders);

        self.jump_was_held = jump_held;
        self.attack_was_held = attack_held;
        self.dash_was_held = dash_held;

        self.sync_network(dt);

        let backdrop_rows = vec![" ".repeat(width.max(1) as usize); height.max(1) as usize];
        let mut sprites = vec![(self.mario, mario_player_color(0), 0usize)];
        for (index, packet) in self.remote.values().enumerate() {
            sprites.push((remote_mario_state(packet), mario_player_color(index + 1), index + 1));
        }
        // The full two-entity pair (slab + its passable underside, with the same light-band gap
        // between them) for rendering -- physics above only ever sees the slab. Mirrors exactly how
        // the native example's own `main.rs` builds its render-side collider list.
        let (px0, _, px1, py1) = self.platform;
        let underside_y0 = py1 + state::MARIO_PLATFORM_LIGHT_BAND_GAP_FRACTION;
        let render_colliders = [
            state::MarioCollider { rect: self.platform, passable: false },
            state::MarioCollider { rect: (px0, underside_y0, px1, underside_y0 + state::MARIO_PLATFORM_UNDERSIDE_HEIGHT_FRACTION), passable: true },
        ];
        mario_body_text(&backdrop_rows, &sprites, &[], None, width, height, &render_colliders)
    }

    fn any_held(&self, codes: &[&str]) -> bool {
        codes.iter().any(|code| self.keys_down.contains(*code))
    }

    fn axis(&self, negative: &str, positive: &str) -> f32 {
        let mut value = 0.0;
        if self.keys_down.contains(negative) {
            value -= 1.0;
        }
        if self.keys_down.contains(positive) {
            value += 1.0;
        }
        value
    }

    /// Sends this player's own position on `SEND_INTERVAL_SECS` (matching the native side's own
    /// cadence) and applies whatever remote positions arrived since the last tick. No-op entirely
    /// if `connect` was never called.
    fn sync_network(&mut self, dt: f32) {
        let Some(relay) = self.relay.as_ref() else { return };

        for text in relay.poll_messages(POSITION_CHANNEL_LABEL) {
            if let Ok(packet) = serde_json::from_str::<PositionPacket>(&text) {
                self.remote.insert(packet.candidate_id.clone(), packet);
            }
        }
        // Combat messages are drained (so the inbox doesn't grow unboundedly) but not applied —
        // see this module's own doc comment on why combat sync isn't wired up this pass.
        let _ = relay.poll_messages(COMBAT_CHANNEL_LABEL);

        self.send_accum += dt;
        if self.send_accum < SEND_INTERVAL_SECS {
            return;
        }
        self.send_accum = 0.0;
        self.seq = self.seq.wrapping_add(1);
        let packet = PositionPacket {
            candidate_id: self.candidate_id.clone(),
            seq: self.seq,
            x: self.mario.x,
            y: self.mario.y,
            vx: self.mario.vx,
            vy: self.mario.vy,
            grounded: self.mario.grounded,
            jumps_used: self.mario.jumps_used,
            alive: self.mario.alive,
        };
        if let Ok(text) = serde_json::to_string(&packet) {
            relay.send(POSITION_CHANNEL_LABEL, &text);
        }
    }
}

impl Default for Game {
    fn default() -> Self {
        Game::new()
    }
}

/// Builds a renderable `MarioState` for a remote peer from their last-known `PositionPacket` — same
/// shape as the native `main.rs`'s own remote-render path (defaults for everything a position
/// packet doesn't carry: no in-flight attack/dust/death effect to show, since those aren't synced).
fn remote_mario_state(packet: &PositionPacket) -> MarioState {
    MarioState {
        x: packet.x,
        y: packet.y,
        prev_x: packet.x,
        prev_y: packet.y,
        vx: packet.vx,
        vy: packet.vy,
        grounded: packet.grounded,
        jumps_used: packet.jumps_used,
        touching_wall: None,
        dust_effect: None,
        facing: 1.0,
        attack_cooldown: 0.0,
        time_since_last_attack: state::MARIO_ATTACK_SPAM_WINDOW + 1.0,
        attack_spam_stacks: 0,
        kills: 0,
        attack_effect: None,
        alive: packet.alive,
        lives: state::MARIO_STARTING_LIVES,
        respawn_remaining: None,
        death_effect: None,
    }
}

/// One frame's worth of gamepad input, read directly from the browser's [Gamepad
/// API](https://developer.mozilla.org/en-US/docs/Web/API/Gamepad_API) — no JS-side wiring needed,
/// this crate polls `navigator.getGamepads()` itself each tick. Only the first connected gamepad is
/// read (single local player, matching this crate's own keyboard handling). Button/axis indices
/// follow the API's "standard" gamepad mapping: `buttons[0]` is the bottom face button (South —
/// jump), `buttons[1]` is the right face button (East — attack), matching the native example's own
/// `bevy_gilrs`-based South/East convention (`physics.rs`'s `MARIO_ATTACK_BUTTON`/jump handling);
/// `axes[0]`/`axes[1]` are the left stick's X/Y axes, `buttons[13]` is the d-pad's down button
/// (crouch), `buttons[4]`/`buttons[5]` are the left/right shoulder bumpers (dash — either one,
/// matching the native example's own `MARIO_DASH_BUTTONS`).
///
/// Read via raw `js_sys::Reflect` property access (`navigator`/`getGamepads`/each pad's fields),
/// not `web_sys::window()`/`Navigator`/`Gamepad`/`GamepadButton`'s typed bindings, deliberately:
/// same reasoning as `atlas-relay`'s own `client_wasm` module's `session_description`/
/// `channel_is_open` (see those doc comments) — `web-sys`'s "Window"/"Navigator" feature set pulls
/// in an unrelated `Intl`-adjacent string enum (`SupportedValuesKey`) that trips the same
/// `wasm-bindgen-cli` 0.2.126 "duplicate string enums" bug, reproduced the same way. Untyped JS
/// property access sidesteps it entirely and needs no extra `web-sys` features on this crate at
/// all.
struct GamepadFrame {
    stick_x: f32,
    stick_y: f32,
    south: bool,
    east: bool,
    down: bool,
    left_bumper: bool,
    right_bumper: bool,
}

fn poll_gamepad() -> GamepadFrame {
    let mut frame =
        GamepadFrame { stick_x: 0.0, stick_y: 0.0, south: false, east: false, down: false, left_bumper: false, right_bumper: false };

    let global = js_sys::global();
    let Ok(navigator) = js_sys::Reflect::get(&global, &JsValue::from_str("navigator")) else { return frame };
    let Ok(get_gamepads) = js_sys::Reflect::get(&navigator, &JsValue::from_str("getGamepads")).and_then(|value| value.dyn_into::<js_sys::Function>())
    else {
        return frame;
    };
    let Ok(pads) = get_gamepads.call0(&navigator).and_then(|value| value.dyn_into::<js_sys::Array>()) else { return frame };

    for entry in pads.iter() {
        if entry.is_null() || entry.is_undefined() {
            continue;
        }
        let connected = js_sys::Reflect::get(&entry, &JsValue::from_str("connected")).ok().and_then(|value| value.as_bool()).unwrap_or(false);
        if !connected {
            continue;
        }

        let Ok(buttons) = js_sys::Reflect::get(&entry, &JsValue::from_str("buttons")).and_then(|value| value.dyn_into::<js_sys::Array>()) else {
            continue;
        };
        let Ok(axes) = js_sys::Reflect::get(&entry, &JsValue::from_str("axes")).and_then(|value| value.dyn_into::<js_sys::Array>()) else { continue };

        frame.south = button_pressed(&buttons, 0);
        frame.east = button_pressed(&buttons, 1);
        frame.down = button_pressed(&buttons, 13);
        frame.left_bumper = button_pressed(&buttons, 4);
        frame.right_bumper = button_pressed(&buttons, 5);

        let stick_x = axes.get(0).as_f64().unwrap_or(0.0) as f32;
        const DEADZONE: f32 = 0.2;
        frame.stick_x = if stick_x.abs() > DEADZONE { stick_x } else { 0.0 };

        // Flipped from the stick's own "up is positive" convention to `MarioState::y`'s "down is
        // positive" one, matching the native example's own `attack_stick_y` -- see `MarioState::
        // step`'s `stick_y` param, shared by attack-aim and the dash burst.
        let stick_y = axes.get(1).as_f64().unwrap_or(0.0) as f32;
        frame.stick_y = if stick_y.abs() > DEADZONE { -stick_y } else { 0.0 };

        // D-pad left/right also move, same as the native example's own d-pad support.
        if button_pressed(&buttons, 14) {
            frame.stick_x = -1.0;
        } else if button_pressed(&buttons, 15) {
            frame.stick_x = 1.0;
        }

        break; // Only the first connected gamepad -- single local player.
    }

    frame
}

fn button_pressed(buttons: &js_sys::Array, index: u32) -> bool {
    let Some(button) = buttons.get(index).dyn_ref::<js_sys::Object>().cloned() else { return false };
    js_sys::Reflect::get(&button, &JsValue::from_str("pressed")).ok().and_then(|value| value.as_bool()).unwrap_or(false)
}
