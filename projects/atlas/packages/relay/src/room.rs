//! Room membership: who's connected, which room they're in, and how to reach them.

use std::collections::HashMap;

use parking_lot::RwLock;
use serde::Deserialize;
use serde::Serialize;
use tokio::sync::mpsc::UnboundedSender;
use uuid::Uuid;

use crate::protocol::ServerMessage;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct PeerId(pub Uuid);

type Peers = HashMap<PeerId, UnboundedSender<ServerMessage>>;

#[derive(Default)]
pub struct Rooms {
    rooms: RwLock<HashMap<String, Peers>>,
}

impl Rooms {
    /// Adds `peer_id` to `room`, broadcasts `PeerJoined` to whoever was already there, and
    /// returns their IDs so the caller can tell the new peer who to connect to.
    pub fn join(&self, room: &str, peer_id: PeerId, sender: UnboundedSender<ServerMessage>) -> Vec<PeerId> {
        let mut rooms = self.rooms.write();
        let peers = rooms.entry(room.to_string()).or_default();

        let existing: Vec<PeerId> = peers.keys().copied().collect();
        for existing_sender in peers.values() {
            let _ = existing_sender.send(ServerMessage::PeerJoined { peer_id });
        }

        peers.insert(peer_id, sender);
        existing
    }

    /// Removes `peer_id` from `room` and broadcasts `PeerLeft` to whoever remains. Drops the
    /// room entirely once it's empty, so a room name doesn't linger forever after everyone's
    /// left it.
    pub fn leave(&self, room: &str, peer_id: PeerId) {
        let mut rooms = self.rooms.write();
        let Some(peers) = rooms.get_mut(room) else { return };

        peers.remove(&peer_id);
        for sender in peers.values() {
            let _ = sender.send(ServerMessage::PeerLeft { peer_id });
        }

        if peers.is_empty() {
            rooms.remove(room);
        }
    }

    /// Forwards `message` to exactly one peer in `room`. Silently does nothing if the room or
    /// peer no longer exists, since a stale target (the other side disconnected mid-handshake)
    /// is a real, expected race, not a server error.
    pub fn send_to(&self, room: &str, target: PeerId, message: ServerMessage) {
        let rooms = self.rooms.read();
        if let Some(sender) = rooms.get(room).and_then(|peers| peers.get(&target)) {
            let _ = sender.send(message);
        }
    }
}
