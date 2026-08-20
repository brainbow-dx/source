//! The wire protocol between a peer and the relay. Plain JSON over the WebSocket's text frames,
//! tagged by `"type"` so both sides can deserialize without knowing which variant to expect.

use serde::Deserialize;
use serde::Serialize;

use crate::room::PeerId;

/// What a peer sends to the relay.
#[derive(Debug, Deserialize)]
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
#[derive(Debug, Serialize)]
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
