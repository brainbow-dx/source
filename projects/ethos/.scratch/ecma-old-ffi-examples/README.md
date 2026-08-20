Old FFI/dashboard demos from the pre-refactor `ecma` dialect (ported 2026-08-14 from
`legacy/examples/ecma`, itself pulled from a RecLab-era backup). Kept as historical reference,
not working examples — the `.example` extension keeps Cargo from ever discovering them as build
targets, intentionally.

- **`chat_webrtc.rs.example`** — not an `ethos-ecma` example at all: a standalone WebRTC
  signaling-server demo (`webrtc`, `tokio-tungstenite`, `base64`) with no reference to the JS
  runtime. Kept here only because it shipped alongside the others; doesn't belong to this dialect.
- **`chat_ffi.rs.example`, `dashboard_ffi.rs.example`** — target the old
  `reclab::runtime::ffi::CScriptRuntimeConfig`/`CConstructRuntimeResultCode`/
  `CSendBroadcastOptions` types, renamed and reshaped since (see `ethos_deno::runtime::ffi`'s
  `CEcmaRuntimeConfig`/`CEcmaRuntime`/`CStartResult` for the current equivalents — no
  result-code wrapper, no broadcast API). Not updated: both demo sending a broadcast message via
  `c_send_broadcast`, which is still a `todo!()` stub in the current runtime, so there's nothing
  real to demonstrate yet.
- **`dashboard_main.js.example`, `dashboard_service.js.example`** — `dashboard_main.js.example`
  is near-byte-identical to `packages/deno/examples/counter/main.js` (confirmed by diffing them),
  which already carries this forward under a new name, sans the one line calling
  `BroadcastChannel`.
