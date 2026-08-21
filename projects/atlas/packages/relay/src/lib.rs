//! A WebRTC signaling relay. Peers connect over a WebSocket, join a named room, and exchange SDP
//! offers/answers and ICE candidates through this server until they've negotiated a direct
//! peer-to-peer connection with each other, at which point this server is no longer involved. It
//! never sees the actual game traffic, only the handshake that sets up the connection.
//!
//! One `Room` is a set of peers that can discover and message each other. A peer's real identity
//! within a room is the `PeerId` this server assigns on join, not anything the client supplies.

pub mod protocol;
pub mod room;

use std::net::SocketAddr;
use std::sync::Arc;

use axum::extract::ws::Message;
use axum::extract::ws::WebSocket;
use axum::extract::ws::WebSocketUpgrade;
use axum::extract::State;
use axum::routing::get;
use axum::Router;

use futures_util::SinkExt;
use futures_util::StreamExt;

use uuid::Uuid;

use protocol::ClientMessage;
use protocol::ServerMessage;
use room::PeerId;
use room::Rooms;

/// Starts the relay, listening on `addr` until the process is killed. `/ws` is the one real
/// route, everything about this server happens over that single WebSocket connection per peer.
pub async fn serve(addr: SocketAddr) -> std::io::Result<()> {
    let rooms = Arc::new(Rooms::default());
    let router = Router::new().route("/ws", get(handle_upgrade)).with_state(rooms);

    let listener = tokio::net::TcpListener::bind(addr).await?;
    tracing::info!("atlas-relay listening on {addr}");

    axum::serve(listener, router).await
}

async fn handle_upgrade(ws: WebSocketUpgrade, State(rooms): State<Arc<Rooms>>) -> axum::response::Response {
    ws.on_upgrade(move |socket| handle_peer(socket, rooms))
}

/// One connected peer's whole lifetime: assign it an ID, relay messages until it disconnects,
/// then clean up its room membership.
async fn handle_peer(socket: WebSocket, rooms: Arc<Rooms>) {
    let peer_id = PeerId(Uuid::new_v4());
    let (mut writer, mut reader) = socket.split();

    // Every message this peer should receive, from another peer or from the room itself,
    // arrives on this channel rather than the room state writing to the socket directly. The
    // socket's write half is only ever touched by this one task below.
    let (outbound_tx, mut outbound_rx) = tokio::sync::mpsc::unbounded_channel::<ServerMessage>();

    let forward_task = tokio::spawn(async move {
        while let Some(message) = outbound_rx.recv().await {
            let Ok(text) = serde_json::to_string(&message) else { continue };
            if writer.send(Message::Text(text.into())).await.is_err() {
                break;
            }
        }
    });

    let mut joined_room: Option<String> = None;

    while let Some(Ok(message)) = reader.next().await {
        let Message::Text(text) = message else { continue };

        let message: ClientMessage = match serde_json::from_str(&text) {
            Ok(message) => message,
            Err(error) => {
                let _ = outbound_tx.send(ServerMessage::Error { message: format!("invalid message: {error}") });
                continue;
            }
        };

        match message {
            ClientMessage::Join { room } => {
                if let Some(previous) = &joined_room {
                    rooms.leave(previous, peer_id);
                }

                let peers = rooms.join(&room, peer_id, outbound_tx.clone());
                joined_room = Some(room);

                let _ = outbound_tx.send(ServerMessage::Joined { peer_id, peers });
            }
            ClientMessage::Offer { to, sdp } => {
                if let Some(room) = &joined_room {
                    rooms.send_to(room, to, ServerMessage::Offer { from: peer_id, sdp });
                }
            }
            ClientMessage::Answer { to, sdp } => {
                if let Some(room) = &joined_room {
                    rooms.send_to(room, to, ServerMessage::Answer { from: peer_id, sdp });
                }
            }
            ClientMessage::IceCandidate { to, candidate } => {
                if let Some(room) = &joined_room {
                    rooms.send_to(room, to, ServerMessage::IceCandidate { from: peer_id, candidate });
                }
            }
        }
    }

    if let Some(room) = &joined_room {
        rooms.leave(room, peer_id);
    }
    forward_task.abort();
}
