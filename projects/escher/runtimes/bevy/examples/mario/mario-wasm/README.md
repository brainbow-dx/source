# mario-wasm

The `mario` example's solo game feel, running in a browser tab via [xterm.js](https://xtermjs.org/)
instead of a real terminal. Keyboard and gamepad both work, and it can join the same LAN session a
native `mario --host`/`--connect` instance is running (position sync only — see "What's cut" below).

## Build and run

```sh
# From this directory (runtimes/bevy/examples/mario/mario-wasm):
cargo build --target wasm32-unknown-unknown --profile wasm-release

# NOT `--release` -- see "The wasm-bindgen-cli bug" below for why this crate needs its own
# profile. `--profile wasm-release` is defined in escher's own top-level `Cargo.toml`.

# wasm-bindgen the CLI must match the `wasm-bindgen` *crate* version exactly (this workspace's
# Cargo.lock currently pins 0.2.126 — check `grep -A2 '"wasm-bindgen"' ../../../../../Cargo.lock`
# if this ever drifts). If `wasm-bindgen --version` doesn't match, install a scoped copy rather
# than overwriting whatever version is already on your PATH:
#   cargo install wasm-bindgen-cli --version 0.2.126 --root /tmp/wasm-bindgen-cli-0.2.126
#   export PATH="/tmp/wasm-bindgen-cli-0.2.126/bin:$PATH"
wasm-bindgen --target web --out-dir web/pkg target/wasm32-unknown-unknown/wasm-release/mario_wasm.wasm

# Serve the static harness (xterm.js is loaded from a CDN `<script>` tag in index.html, everything
# else is local) -- a plain file:// open won't work, browsers block ES module imports from file://.
cd web && python3 -m http.server 8934
# then open http://127.0.0.1:8934/
# ?relay=ws://<host>:9200/ws and ?room=<name> override the defaults (a relay on this same
# machine, room "mario") to join a specific LAN session -- see "Networking" below.
```

Controls: arrow keys/WASD/gamepad left stick or d-pad to move, up/space/W/gamepad South to jump
(hold for a full jump, tap for a short one — double-tap for a double jump, or off a wall for a
wall kick), X/J/gamepad East to attack, down/S/gamepad d-pad-down to crouch.

## How it's built

- **`mario-core`** (sibling crate): the actual game feel — `MarioState::step` (movement, gravity,
  jumps, collision, attack timing) and `mario_body_text` (the ANSI-colored terminal-grid renderer).
  Bevy-free, so it compiles for both the native example and `wasm32-unknown-unknown`.
- **`mario-wasm`** (this crate): a thin `wasm-bindgen` adapter — `Game::setKey` takes keyboard
  state from JS, `Game::tick` steps the simulation (also polling gamepad state directly, see
  `poll_gamepad`) and returns one rendered frame as a string, `Game::connect` joins a relay room.
- **`atlas-relay`** (`projects/atlas/packages/relay`): now also the home of the wasm32 side of the
  networking, not just the native signaling server -- see "Networking" below.
- **`web/index.html`**: loads xterm.js from a CDN, forwards real `keydown`/`keyup` events (not
  xterm's own `onData` — a held-vs-tapped distinction is needed for pressure-sensitive jumping,
  which a one-shot text event can't give), hides xterm's own text cursor (`\x1b[?25l` — `cursorBlink:
  false` alone doesn't do this, it only stops the blink), and writes each frame with a cursor-home
  escape rather than a full clear (the renderer already repaints every cell every frame).

## Networking

Joins the exact same `atlas-relay` room a native `mario --host`/`--connect` session uses, so a
browser tab and a native peer see each other move. The signaling protocol
(`atlas_relay::protocol::{ClientMessage, ServerMessage}`, plain JSON over a WebSocket) is already
browser-friendly; only the actual peer-to-peer transport needed a wasm32-specific implementation,
since `webrtc-rs` (what the native `relay.rs` uses) doesn't target wasm32.

That implementation lives in **`atlas-relay` itself**, not a separate crate: `atlas-relay/src/
client_wasm.rs`, a `#[cfg(target_arch = "wasm32")]`-gated module using `web-sys`'s browser-native
`RtcPeerConnection`/`RtcDataChannel`/`WebSocket`. The crate's `Cargo.toml` moved its `axum`/`tokio`
server dependencies into a `[target.'cfg(not(target_arch = "wasm32"))'.dependencies]` table and
added a wasm32-only one for `web-sys`/`wasm-bindgen-futures`/`js-sys`, so a native consumer
(`escher-bevy`'s own `relay.rs`) and a wasm32 one (this crate) both just depend on plain
`atlas-relay` and each gets the right half — no second crate needed to keep the wasm side from
dragging in a native-only server. `protocol.rs`'s wire types (including `PeerId`, moved there from
`room.rs` for exactly this reason) compile on every target; `room.rs` (the native `Rooms`
membership tracker) and `client_wasm.rs` are each gated to the target that actually needs them.

**Position sync only, this pass.** `PositionPacket` here (`mario-wasm/src/lib.rs`) is a deliberate,
disclosed duplicate of `relay.rs`'s own struct of the same name: identical fields/serde shape so
JSON round-trips between a browser peer and a native one, but there's no way to depend on it
directly since `relay.rs` lives inside the native `mario` example binary, not a library crate. A
remote peer's motion is sent/received and rendered as an extra sprite, the same way the native
`main.rs`'s own remote-render path works. The combat data channel is opened (so a native peer's own
`on_data_channel` sees a recognized label) but never used to send or apply a `CombatEvent` — a
browser player can't hit or be hit yet. Real, scoped future work: porting `physics.rs`'s
attack-cone-hits logic into `mario-core` the same way movement/collision already were.

## The wasm-bindgen-cli bug (real, reproduced, and why this crate needs its own build profile)

`wasm-bindgen-cli` 0.2.126 -- the exact version this workspace is pinned to (see "Build and run"
above) -- has a real bug: post-processing a wasm32 binary that references more than one variant of
certain `web-sys` "string enum" types (confirmed for `RtcSdpType`, `RtcDataChannelType`,
`RtcDataChannelState`) fails outright with `error: duplicate string enums`, seeing the same
generated aux type as registered twice. Reproduced and isolated methodically, not guessed: ruled
out call-site count, `#[inline(never)]`, and optimization level individually before finding the
real pattern. Fixed at the source for every case actually found: `client_wasm.rs`'s
`session_description`/`channel_is_open` and `mario-wasm`'s own `poll_gamepad`/`button_pressed`
build/read the relevant JS objects via raw `js_sys::Reflect`/`Object` calls instead of `web-sys`'s
typed enum-returning methods, so the problematic type is never named in Rust code at all --
`web-sys` features for the affected types (`RtcSdpType`, `RtcDataChannelType`,
`RtcDataChannelState`, `Window`, `Navigator`, `Gamepad`, `GamepadButton`) were removed accordingly.

That alone wasn't enough, though: with this workspace's own `[profile.release]` (`lto = true`,
`codegen-units = 1`), a *different* enum (`SupportedValuesKey`, an `Intl`-adjacent type pulled in
transitively by other still-needed `web-sys` features like `console`/`WebSocket`) tripped the same
bug -- consistent with LTO/single-codegen-unit inlining duplicating an aux-type-registration thunk
across crate boundaries in a way smaller or less-optimized builds don't hit. Rather than turn off
this workspace's LTO globally (a real regression for every other release build, e.g. Anvil's), a
separate named profile (`[profile.wasm-release]` in `escher/Cargo.toml`, `inherits = "dev"`,
`opt-level = 1` for the current crate, `opt-level = 3` for dependencies via a package override) is
used for this one build path via `--profile wasm-release`. Real optimization for dependencies, just
not LTO'd/single-unit for the crates actually calling into `web-sys`. Tradeoff, stated plainly: the
resulting `.wasm` (~545KB before gzip) is larger than a full LTO release build would produce --
acceptable for a dev-facing example, worth revisiting if this ever ships more broadly.

## What's cut, on purpose, this pass

- **No ghosts, sound, or pause menu.** All three are either not meaningfully useful without
  persistence/multiplayer (ghosts, which persist across sessions via `sqld`) or were simply out of
  scope for "get a browser tab playable" — none is a technical blocker, just cut for time.
- **Combat isn't synced** — see "Networking" above.

## Honest verification status

Compiles clean for both the host target and `wasm32-unknown-unknown` (`cargo check -p mario-core -p
mario-wasm`, `cargo build -p mario-wasm --target wasm32-unknown-unknown --profile wasm-release`),
and the native `mario` example (`cargo check --workspace --exclude escher-unity --exclude
escher-unreal --examples --features escher-bevy/audio`) is unaffected -- confirmed via a real build
after the `atlas-relay` restructuring, not assumed. `atlas-relay`'s own existing test
(`two_peers_relay_a_handshake`) still passes.

Real browser-verified (Chrome browser automation): movement, boundary-clamping, and attack-
triggering all work with no console errors; a real jump bug was found and fixed by testing the
actual running build (see git history / changelog for the platform-height fix). The gamepad and
networking code added this pass compiles clean and (for networking) is architecturally sound and
protocol-compatible with the native side, but **has not been confirmed live against a real gamepad
or a real running LAN session from this session** -- that's the next real check, on whoever picks
this up next.
