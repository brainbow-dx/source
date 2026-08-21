# A generic WebRTC client for `atlas-relay`, replacing per-consumer boilerplate

Status: planned 2026-08-20, not started. Raised directly ("building out a relay is a lot of boilerplate... I'd like it to be as simple as defining startup and config and then passing handlers for different requests/events/whatever") while extending `runtimes/bevy/examples/mario`'s relay sync with a second data channel for combat events. This doc is the concrete implementation plan the existing `direct-peer-connections-threshold.md` already flagged as "not started" under its 2026-08-18 update — that doc is the decision-level record (why a generic WebRTC transport belongs in the Atlas SDK at all); this one is the API design and migration plan.

## The problem, concretely

`escher/runtimes/bevy/examples/mario/relay.rs` is ~450 lines. Once the position-sync-only version is set aside, essentially none of it is specific to Mario:

- `PeerConnectionEventHandler` impl (ICE candidate forwarding, routing an inbound data channel by label)
- `IceGate` (buffering ICE candidates that arrive before the remote description is set — a real race, already found and fixed once)
- The newcomer-offers-first negotiation rule, and its offer/answer/ICE-candidate relay over the signaling socket
- `connect_with_retry` (a fresh connect racing a not-yet-ready relay — also a real, found bug)
- One hand-written `spawn_X_send_loop`/`spawn_X_receive_loop` pair per data channel, differing only in the payload type and reliability settings

Two channels (position, combat) already exist. A third consumer (Hudd's strokes, a tracing stream, a chat channel) means copying this file a third time, each copy able to drift from the others' bug fixes independently — `connect_with_retry`'s fix, for instance, would need to be reapplied by hand to every future copy.

## Goal

A caller should be able to write roughly this, not re-derive peer connection setup:

```rust
let relay = atlas_relay::client::Builder::new(relay_url, room)
    .channel::<PositionPacket>("position", Reliability::BestEffort, |peer_id, packet| { ... })
    .channel::<CombatEvent>("combat", Reliability::Reliable, |peer_id, event| { ... })
    .on_peer_joined(|peer_id| { ... })
    .on_peer_left(|peer_id| { ... })
    .spawn(runtime_handle);

relay.send("position", &packet); // broadcasts to every connected peer's "position" channel
```

Startup/config (the relay URL, room, which channels exist and their reliability) is declared once; everything else — negotiation, ICE, per-channel plumbing — is the library's problem, not the caller's.

## Where this lives

Not unconditionally inside `atlas-relay` as it exists today. That crate currently has **zero** `webrtc` dependency (confirmed via `Cargo.toml` and a `Cargo.lock` grep, both current) — it's purely the signaling server (`serve`, `protocol`, `room`). Baking a WebRTC peer-connection client into it unconditionally would force `webrtc`/`tokio-tungstenite`/`async-trait` onto every consumer that only ever runs the signaling server (Anvil's own `spawn_relay_server`, the `compose.yaml` `relay` service, `atlas-dev`) — the same "don't force heft nobody asked for" lesson this session already applied to `escher-bevy`'s new `audio` feature.

Plan: a new `client` module in `atlas-relay`, gated behind a new Cargo feature of the same name (off by default), with `webrtc`, `tokio-tungstenite`, and `async-trait` becoming `optional = true` dependencies enabled only by that feature. `futures-util` stays a mandatory dependency either way (the signaling server side already uses it). Consumers that only run the server keep depending on `atlas-relay` with default features; a client consumer (Mario, and later Hudd) depends on `atlas-relay = { features = ["client"] }`.

## API sketch

```rust
pub mod client {
    pub enum Reliability {
        /// Unordered, zero-retransmit. For continuously-resent state where a fresher message is
        /// always moments away and a dropped one is a non-event (today's position packets).
        BestEffort,
        /// Reliable, ordered — the default `RTCDataChannelInit`. For one-shot events where a lost
        /// message matters (today's combat events: a dropped "you died" permanently desyncs who's
        /// alive, unlike a dropped position tick).
        Reliable,
    }

    pub struct Builder { /* relay_url, room, channel specs, peer-lifecycle handlers */ }

    impl Builder {
        pub fn new(relay_url: impl Into<String>, room: impl Into<String>) -> Self;

        /// Registers a channel: every peer gets one `RTCDataChannel` with this label and
        /// reliability. `handler` fires once per message received on it from any peer, already
        /// deserialized — `T` only needs `Serialize + DeserializeOwned + Send + 'static`, the same
        /// bound `PositionPacket`/`CombatEvent` already satisfy today.
        pub fn channel<T>(self, label: impl Into<String>, reliability: Reliability, handler: impl Fn(PeerId, T) + Send + Sync + 'static) -> Self
        where T: serde::Serialize + serde::de::DeserializeOwned + Send + 'static;

        pub fn on_peer_joined(self, handler: impl Fn(PeerId) + Send + Sync + 'static) -> Self;
        pub fn on_peer_left(self, handler: impl Fn(PeerId) + Send + Sync + 'static) -> Self;

        /// Connects (with the same retry-on-not-ready-yet behavior `connect_with_retry` already
        /// proved out) and starts every registered channel's negotiation. Returns immediately;
        /// everything after this runs on `runtime`.
        pub fn spawn(self, runtime: tokio::runtime::Handle) -> Client;
    }

    pub struct Client { /* handle for sending; owns nothing the caller needs to poll */ }

    impl Client {
        /// Serializes `message` and sends it to every currently connected peer's channel named
        /// `label`. Broadcasts, same as both of Mario's existing send loops do today — a future
        /// send-to-one-peer variant is a real gap (see "Deliberately deferred" below), not
        /// something this call silently only-sort-of does.
        pub fn send<T: serde::Serialize>(&self, label: &str, message: &T);
    }
}
```

## Internal architecture (generalizing what already works)

Reuse the proven pieces from `relay.rs` directly, generalized:

- `PeerLink.position_channel`/`combat_channel` (two named fields) → `PeerLink.channels: HashMap<String, RwLock<Option<Arc<dyn DataChannel>>>>`, one entry per registered `channel()` call, still created eagerly by the offering side (`create_data_channel` once per spec, in `open_offer`) and routed by label in the answering side's `on_data_channel` (already how the position/combat split works today — this generalizes to N labels instead of a 2-arm `match`).
- `IceGate`, the newcomer-offers rule, `flush_pending_ice`, `connect_with_retry`: unchanged, they were already fully generic — the position/combat split never touched any of this.
- Per-channel send loops collapse into one generic loop parameterized by `(label, Reliability)`: `BestEffort` channels get the existing fixed-interval "always resend current value" loop (needs a caller-supplied "what's the current value" callback, or the caller just calls `send()` on its own timer — simpler, and matches how `spawn_send_loop`'s `SEND_INTERVAL` ticking is really Mario's own choice, not something the transport should impose); `Reliable` channels just need `send()` to go out immediately, no interval loop at all.
- Per-channel receive loops collapse into one generic loop that deserializes into the registered handler's `T` and calls it with the sending peer's id — today's `remote_mario`/`incoming_combat` tables are Mario's own choice of what to do with a receive, not something the transport should hardcode; the caller's `channel::<T>` handler closure replaces both.

## Migration plan

1. Build `atlas_relay::client` as above, with real unit/integration tests (`atlas-relay/tests/` already has a real relay-server integration test to extend, per `tests/relay.rs`) — not just compiling, actually negotiating a connection and passing a message end to end, the same bar `host_actions.rs`'s test held itself to for the unrelated Ethos op work.
2. Migrate `examples/mario/relay.rs` onto it as the first real consumer: two `channel::<T>()` registrations (`PositionPacket` best-effort, `CombatEvent` reliable) replacing the whole file's hand-rolled version. This is the actual proof the abstraction holds — if Mario's real, already-working behavior doesn't come out the same after migration, the API isn't right yet.
3. Only after step 2 is verified working (a real build + the same LAN playtest this was raised during, run again) does this count as done. No second consumer is required to land this — Hudd/tracing-stream adoption is real motivation but not a blocking dependency of this plan.

## Deliberately deferred, not part of this plan

- **Send-to-one-peer**: today's transport (and this design) only broadcasts. A future need to address one specific peer (not everyone in the room) isn't designed here.
- **Backpressure/ordering across channels**: each channel is independent; no cross-channel ordering guarantee is provided or planned.
- **Non-Mario consumers**: Hudd's strokes, a tracing stream, etc. are the motivating "why generalize" cases from `direct-peer-connections-threshold.md`, but no second consumer's actual shape has been scoped — this plan only commits to the API being generic enough in principle, not to a second caller existing yet.

## Explicitly not blocking tonight

Tonight's LAN playtest (`escher/mario-platform-polish`) keeps using the current hand-rolled `relay.rs` as-is — this is a real but separate follow-up, scoped here so it doesn't get lost, not something to fold into or block the in-flight CLI-ergonomics work (`--host`/`--connect`) happening in parallel.
