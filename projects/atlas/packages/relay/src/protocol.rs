//! The wire protocol between a peer and the relay. Plain JSON over the WebSocket's text frames,
//! tagged by `"type"` so both sides can deserialize without knowing which variant to expect.
//!
//! Both enums derive both `Serialize` and `Deserialize` (not just the one direction this crate's
//! own relay server needs) so a real peer -- which sends `ClientMessage` and receives
//! `ServerMessage`, the opposite of the relay -- can depend on this crate for the wire types
//! directly instead of hand-rolling a second copy of the protocol against raw JSON strings. Both
//! the native peer (`escher`'s `mario` example, `relay.rs`) and the wasm32 browser peer (this
//! crate's own `client_wasm` module) do exactly that.

use serde::Deserialize;
use serde::Serialize;
use uuid::Uuid;

/// A peer's identity within a room, assigned by the relay server on join -- never supplied by the
/// peer itself. Lives here, not `room.rs`, since `room.rs`'s `Rooms` is native-only (uses
/// `tokio::sync::mpsc`), while this type has to stay available on every target -- both the native
/// peer path and the wasm32 `client_wasm` module need it, and both already depend on this module
/// for `ClientMessage`/`ServerMessage`. `room.rs` re-exports it under its own existing path so no
/// native call site (`atlas_relay::room::PeerId`) needs to change.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub Uuid);

/// What a peer sends to the relay.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ClientMessage {
    /// Join (or move to) a room by name. The relay assigns this connection a `PeerId` on its
    /// first join, regardless of which room, so a peer keeps the same identity across rooms.
    Join { room: String },
    /// An SDP offer for `to`, forwarded verbatim, opaque to this server.
    Offer { to: PeerId, sdp: String },
    /// An SDP answer for `to`, forwarded verbatim.
    Answer { to: PeerId, sdp: String },
    /// One ICE candidate for `to`, forwarded verbatim.
    IceCandidate { to: PeerId, candidate: String },
}

/// What the relay sends to a peer.
#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "kebab-case")]
pub enum ServerMessage {
    /// Sent once, right after a successful `Join` — this connection's own ID, plus every peer
    /// already in the room (so the new peer knows who to open connections to).
    Joined { peer_id: PeerId, peers: Vec<PeerId> },
    /// A peer joined the room this connection is currently in.
    PeerJoined { peer_id: PeerId },
    /// A peer left the room this connection is currently in (disconnected, or joined another
    /// room).
    PeerLeft { peer_id: PeerId },
    Offer { from: PeerId, sdp: String },
    Answer { from: PeerId, sdp: String },
    IceCandidate { from: PeerId, candidate: String },
    Error { message: String },
}
