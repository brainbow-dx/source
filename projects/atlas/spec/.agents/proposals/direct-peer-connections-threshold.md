# Atlas SDK: sqld-sync and WebRTC as parallel transports, not a toggle

Status: decided 2026-08-17, in conversation — recorded here so it doesn't get lost the way an earlier Anvil investigation did (see Escher's `spec/ROADMAP.md` M1 for that story). Corrected the same day: an earlier version of this doc framed this as one transport replacing the other past some usage "cap." That's wrong — see below.

## The decision

The Atlas SDK offers **both** transports permanently, side by side. A feature picks whichever one fits its own data's characteristics — this isn't a threshold that gets crossed once and then everything moves over; it's two tools that coexist indefinitely, chosen per feature.

- **sqld-sync** (a libsql embedded replica against a remote `sqld` — what Anvil's `persistence` module already does) is fine for coordination-shaped data: chat, task lists, anything low-frequency and small where "synced within a second or two, read from a local replica" is the actual requirement, not a limitation to route around.
- **Atlas SDK's WebRTC features** are for data that's fundamentally the wrong shape for a synced database in the first place, regardless of volume — large tracing streams, buffered video, drawing/annotation line buffers (e.g. Hudd's strokes), file transfers. These want direct, low-latency peer delivery from the start, not "sqld until it becomes a problem."

The dividing line is the data's own shape (streamed/high-bandwidth/latency-sensitive vs. coordination-shaped), decided per feature — not a global switchover point.

## The transport

`packages/relay` (`atlas-relay`) already has the real, tested WebRTC signaling half (`PeerId`, room join/leave presence, `Offer`/`Answer`/`IceCandidate` forwarding — see `protocol.rs`). Confirmed via a full `Cargo.lock` grep the same day: there is currently zero `webrtc`/`webrtc-rs` dependency anywhere in the workspace, so the actual `RTCPeerConnection`/data-channel client that would consume that signaling doesn't exist yet — real, from-scratch work, not a small addition on top of what's already there.

Tailscale (already the planned dev-time transport for peer addressing generally — see the sibling discussion on peer addressing) removes the STUN/TURN half of that work during dev, not the ICE-negotiation/data-channel integration itself.

**Update, 2026-08-18**: this from-scratch client now exists, once, but in the wrong place. Escher's `runtimes/bevy/examples/mario/relay.rs` (formerly Anvil's `peer_sync.rs`) is a real, working `webrtc`-backed data channel client negotiated over `atlas-relay`'s signaling socket, including the offer/answer/ICE-candidate flow and a fix for a real ICE-arrives-before-remote-description race. Everything in it except the position packet's own field shape and its channel label is generic: peer connection setup, the newcomer-offers rule, ICE buffering. That generic part should become the Atlas SDK's WebRTC transport this doc describes, parameterized over an arbitrary serializable payload type and channel label, so a future second consumer (Hudd's strokes, a tracing stream) gets it for free instead of copying the same file again. Not started.

## Provisioning (future, not started)

Stated intent, not yet designed: an Anvil extension, scripted via its existing Deno command runtime (the same pattern `/shape` already establishes — see `apps/anvil/commands/`), to spin up a small Tailscale-tagged service per host, plus whatever other services the mesh needs. No further shape decided yet (what triggers provisioning, service lifecycle, tag scheme) — recorded here only so the intent itself isn't lost before it's designed.
