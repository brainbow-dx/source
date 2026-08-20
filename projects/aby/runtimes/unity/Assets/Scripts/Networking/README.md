# P2P networking (2026-08-14)

Real peer-to-peer connectivity over WebRTC, signaled through `atlas-relay` (`atlas/packages/relay`, verified separately — see its own tests). This is the connectivity layer only: two or more clients can find each other and open a data channel. Actual game-state replication (player position, animation, etc.) on top of that channel is separate, not-yet-built work.

## What's unverified, honestly

None of the C# in this folder has been compiled or run — there's no Unity Editor or browser drivable from the environment this was written in. What's real and independently verified: the exact JSON wire protocol these scripts speak (`SignalingProtocol.cs`/`ServerMessage.cs`) matches `atlas-relay`'s own protocol, proven by a real integration test in `atlas/packages/relay/tests/relay.rs` that drives the actual server with real WebSocket clients. Everything downstream of that (the two WebRTC backends, `NativeWebRtcTransport.cs` and `WebGLWebRtcTransport.cs`/`WebRtcBridge.jslib`) was written carefully against the documented shape of `com.unity.webrtc`'s API and the browser's `RTCPeerConnection` API, but needs real Editor/browser testing to confirm it actually works.

## How to verify

1. Start the relay: `cargo run -p atlas-relay --bin atlas-relay -- 0.0.0.0:9200` (from `projects/atlas`).
2. Open the project in Unity 6000.3.8f1 (or newer). Let package resolution finish, resolving `com.unity.webrtc` and `com.endel.nativewebsocket` for the first time will take a while.
3. Put a `NetworkPlayer` component on a `GameObject` in a scene, run it in two Editor instances (ParrelSync, already a package dependency here, is the easy way to get a second Editor instance without a full separate checkout) or one Editor plus one standalone/WebGL build. Both should default to `ws://localhost:9200/ws` and room `aby-dev`.
4. Watch the Console on both: `[NetworkPlayer] peer joined`, then (once the WebRTC handshake actually completes) `[NetworkPlayer] data channel open to <id>, sending hello`, then `[NetworkPlayer] received from <id>: hello from <id>` on each side. That full sequence landing on both instances is the real proof this works end to end.
