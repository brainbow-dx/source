//! Real-time position and combat sync between running instances, over WebRTC data channels
//! negotiated through `atlas-relay`'s signaling WebSocket. Two data channels per peer, deliberately
//! different reliability characteristics for deliberately different kinds of data:
//!
//! - `POSITION_DATA_CHANNEL_LABEL`: unordered, zero-retransmit. Each packet is a full snapshot of
//!   one player's motion state rather than a delta, and carries a per-sender `seq` so a future
//!   authoritative server could reject stale updates. Sent on a fixed interval: a fresher packet is
//!   always moments away, so a dropped or out-of-order one simply doesn't matter, and no
//!   reliable/ordered channel's head-of-line blocking can add latency to it.
//! - `COMBAT_DATA_CHANNEL_LABEL`: reliable, ordered (the default `RTCDataChannelInit`). A landed
//!   hit is a one-shot event, not continuously-resent state like position — losing one permanently
//!   desyncs "is this player alive" between the two machines, so unlike position it has to arrive.
//!
//! Combat authority is attacker-side: the instance whose local player's swing connects against a
//! remote-tracked target's last-known position decides the hit happened and sends a `CombatEvent`
//! naming the target; the target's own instance is what actually applies the death to its own
//! locally-owned player (see `physics::update_mario_physics`) — the attacker's instance has no
//! authority to mutate a player it doesn't own.
//!
//! Negotiation follows one rule to avoid a two-sided offer race: whoever the relay's `Joined`
//! message names as already-present peers gets offered a connection by the newcomer, and whoever
//! joins after this instance is already in the room instead sends this instance an offer.
//!
//! Networking failures anywhere in this module (no relay running, a peer connection failing
//! mid-handshake, malformed SDP or ICE from a misbehaving peer) are logged and swallowed. Local,
//! single-player movement keeps working with no relay reachable at all.

use std::collections::HashMap;
use std::sync::Arc;
use std::time::Duration;

use atlas_relay::protocol::ClientMessage;
use atlas_relay::protocol::ServerMessage;
use atlas_relay::room::PeerId;

use futures_util::SinkExt;
use futures_util::StreamExt;

use parking_lot::RwLock;

use tokio_tungstenite::tungstenite::Message;

use webrtc::data_channel::DataChannel;
use webrtc::data_channel::DataChannelEvent;
use webrtc::data_channel::RTCDataChannelInit;
use webrtc::error::Result as RtcResult;
use webrtc::peer_connection::PeerConnection;
use webrtc::peer_connection::PeerConnectionBuilder;
use webrtc::peer_connection::PeerConnectionEventHandler;
use webrtc::peer_connection::RTCConfigurationBuilder;
use webrtc::peer_connection::RTCIceCandidateInit;
use webrtc::peer_connection::RTCPeerConnectionIceEvent;
use webrtc::peer_connection::RTCSessionDescription;

/// One player's full motion state at a single instant.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct PositionPacket {
    pub candidate_id: String,
    pub seq: u32,
    pub x: f32,
    pub y: f32,
    pub vx: f32,
    pub vy: f32,
    pub grounded: bool,
    pub jumps_used: u8,
}

/// Every remote peer's most recently received position, keyed by their `candidate_id`. Written
/// only by this module as packets arrive. Never pruned, so a peer that goes quiet stays at its
/// last known position rather than disappearing.
pub type RemoteMarioTable = Arc<RwLock<HashMap<String, PositionPacket>>>;

/// One landed hit, attacker-authoritative (see this module's own doc comment). `dx`/`dy` are the
/// attacking swing's own direction (`MarioAttackEffect::dx`/`dy`), carried along so the target's
/// own instance can build the same reversed death-burst direction the local-vs-local path already
/// does, without needing to know anything about the attacker's actual position.
#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct CombatEvent {
    pub attacker_candidate_id: String,
    pub target_candidate_id: String,
    pub heavy: bool,
    pub dx: f32,
    pub dy: f32,
}

/// About 30Hz, frequent enough that a dropped packet is a non-event.
const SEND_INTERVAL: Duration = Duration::from_millis(33);
const POSITION_DATA_CHANNEL_LABEL: &str = "mario-sync";
const COMBAT_DATA_CHANNEL_LABEL: &str = "mario-combat";

/// One connected peer's negotiated transport: one channel per label (see this module's own doc
/// comment for why position and combat get separate channels with different reliability). Both
/// start `None` on the answering side, since the offerer's `create_data_channel` calls are what
/// open them, which only reach this side later as `PeerConnectionEventHandler::on_data_channel`
/// callbacks, one per channel, routed by `data_channel.label()`. Both are filled in immediately on
/// the offering side, which already holds the channel handles directly.
///
/// `ice_gate` buffers ICE candidates that arrive before the remote description is set. Candidates
/// arrive over the relay socket as independent messages with no ordering guarantee against the
/// task that sets the remote description, so a candidate can arrive before `add_ice_candidate` has
/// anything to attach it to. A single mutex covers both the flag and the buffer so "check the flag,
/// then buffer" and "flip the flag, then drain the buffer" can never interleave.
struct PeerLink {
    connection: Arc<dyn PeerConnection>,
    position_channel: RwLock<Option<Arc<dyn DataChannel>>>,
    combat_channel: RwLock<Option<Arc<dyn DataChannel>>>,
    ice_gate: parking_lot::Mutex<IceGate>,
}

#[derive(Default)]
struct IceGate {
    remote_description_set: bool,
    pending: Vec<RTCIceCandidateInit>,
}

/// State shared by a peer connection's own event handler and the relay's read loop: the relay
/// socket (to forward gathered ICE candidates), the peer table, and the tables incoming positions
/// and combat events get written into.
struct Context {
    ws_sink: WsSink,
    peers: RwLock<HashMap<PeerId, PeerLink>>,
    remote_mario: RemoteMarioTable,
    incoming_combat: Arc<RwLock<Vec<CombatEvent>>>,
}

type WsStream = tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>;
/// An async mutex, not `parking_lot`'s. Sending through the relay socket has to hold this across an
/// `.await`, which a sync lock can't do without risking blocking the runtime while held.
type WsSink = Arc<tokio::sync::Mutex<futures_util::stream::SplitSink<WsStream, Message>>>;

/// Forwards one gathered ICE candidate, or a freshly opened inbound data channel, back through the
/// relay for one peer connection. One of these is built per peer connection, offerer and answerer
/// alike.
struct RelayHandler {
    peer_id: PeerId,
    context: Arc<Context>,
}

#[async_trait::async_trait]
impl PeerConnectionEventHandler for RelayHandler {
    async fn on_ice_candidate(&self, event: RTCPeerConnectionIceEvent) {
        let candidate = match event.candidate.to_json() {
            Ok(init) => init.candidate,
            Err(error) => {
                tracing::warn!("mario relay: failed to encode a gathered ice candidate: {error}");
                return;
            }
        };
        send_client_message(&self.context.ws_sink, &ClientMessage::IceCandidate { to: self.peer_id, candidate }).await;
    }

    /// Only fires on the answering side, once per channel the offerer created. The offerer already
    /// holds its own data channel handles directly from `create_data_channel`'s return values.
    async fn on_data_channel(&self, data_channel: Arc<dyn DataChannel>) {
        let label = data_channel.label().await.unwrap_or_default();
        match label.as_str() {
            POSITION_DATA_CHANNEL_LABEL => {
                if let Some(link) = self.context.peers.read().get(&self.peer_id) {
                    *link.position_channel.write() = Some(data_channel.clone());
                }
                spawn_position_receive_loop(self.peer_id, data_channel, self.context.remote_mario.clone());
            }
            COMBAT_DATA_CHANNEL_LABEL => {
                if let Some(link) = self.context.peers.read().get(&self.peer_id) {
                    *link.combat_channel.write() = Some(data_channel.clone());
                }
                spawn_combat_receive_loop(self.peer_id, data_channel, self.context.incoming_combat.clone());
            }
            other => tracing::warn!("mario relay: {:?} opened a data channel with an unrecognized label {other:?}", self.peer_id),
        }
    }
}

async fn send_client_message(sink: &WsSink, message: &ClientMessage) {
    let Ok(text) = serde_json::to_string(message) else {
        tracing::warn!("mario relay: failed to encode an outgoing relay message");
        return;
    };
    let mut sink = sink.lock().await;
    if let Err(error) = sink.send(Message::Text(text.into())).await {
        tracing::warn!("mario relay: failed to send to the relay: {error}");
    }
}

/// A fresh peer connection with no ICE servers configured. Correct for same-host or LAN testing,
/// where host candidates alone are enough to connect. Peers on separate networks with no STUN or
/// TURN reachable between them won't connect.
async fn new_peer_connection(peer_id: PeerId, context: Arc<Context>) -> RtcResult<Arc<dyn PeerConnection>> {
    let configuration = RTCConfigurationBuilder::default().build();
    let handler = Arc::new(RelayHandler { peer_id, context });

    let connection = PeerConnectionBuilder::new()
        .with_configuration(configuration)
        .with_handler(handler)
        .with_udp_addrs(vec!["0.0.0.0:0"])
        .build()
        .await?;

    Ok(Arc::new(connection) as Arc<dyn PeerConnection>)
}

/// Unordered, zero-retransmit: a stale position packet should never block a fresher one instead of
/// being retransmitted.
fn position_channel_init() -> RTCDataChannelInit {
    RTCDataChannelInit { ordered: false, max_retransmits: Some(0), ..Default::default() }
}

/// This instance is the newcomer to `peer_id`'s room membership, so this instance offers, creating
/// both data channels (see this module's own doc comment for why there are two).
async fn open_offer(peer_id: PeerId, context: Arc<Context>) {
    let connection = match new_peer_connection(peer_id, context.clone()).await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::warn!("mario relay: failed to open a connection to {peer_id:?}: {error}");
            return;
        }
    };

    let position_channel = match connection.create_data_channel(POSITION_DATA_CHANNEL_LABEL, Some(position_channel_init())).await {
        Ok(data_channel) => data_channel,
        Err(error) => {
            tracing::warn!("mario relay: failed to create a position data channel to {peer_id:?}: {error}");
            return;
        }
    };
    let combat_channel = match connection.create_data_channel(COMBAT_DATA_CHANNEL_LABEL, None).await {
        Ok(data_channel) => data_channel,
        Err(error) => {
            tracing::warn!("mario relay: failed to create a combat data channel to {peer_id:?}: {error}");
            return;
        }
    };
    spawn_position_receive_loop(peer_id, position_channel.clone(), context.remote_mario.clone());
    spawn_combat_receive_loop(peer_id, combat_channel.clone(), context.incoming_combat.clone());
    context.peers.write().insert(
        peer_id,
        PeerLink {
            connection: connection.clone(),
            position_channel: RwLock::new(Some(position_channel)),
            combat_channel: RwLock::new(Some(combat_channel)),
            ice_gate: parking_lot::Mutex::new(IceGate::default()),
        },
    );

    let offer = match connection.create_offer(None).await {
        Ok(offer) => offer,
        Err(error) => {
            tracing::warn!("mario relay: failed to create an offer for {peer_id:?}: {error}");
            return;
        }
    };
    let sdp = offer.sdp.clone();
    if let Err(error) = connection.set_local_description(offer).await {
        tracing::warn!("mario relay: failed to set the local description offered to {peer_id:?}: {error}");
        return;
    }

    send_client_message(&context.ws_sink, &ClientMessage::Offer { to: peer_id, sdp }).await;
}

/// Marks `peer_id`'s remote description as set and applies every ICE candidate buffered before this
/// point. Called the instant either side's remote description lands: the offerer's on receiving the
/// answer, the answerer's right after accepting the offer.
async fn flush_pending_ice(peer_id: PeerId, context: &Arc<Context>) {
    let (connection, pending) = {
        let peers = context.peers.read();
        let Some(link) = peers.get(&peer_id) else { return };
        let mut gate = link.ice_gate.lock();
        gate.remote_description_set = true;
        (link.connection.clone(), std::mem::take(&mut gate.pending))
    };

    for candidate in pending {
        if let Err(error) = connection.add_ice_candidate(candidate).await {
            tracing::warn!("mario relay: failed to apply a buffered ice candidate from {peer_id:?}: {error}");
        }
    }
}

/// `peer_id` just joined a room this instance was already in and offered, so this instance
/// answers. The data channel itself arrives later, asynchronously, via `RelayHandler::
/// on_data_channel` once `peer_id`'s own `create_data_channel` call opens it.
async fn accept_offer(peer_id: PeerId, sdp: String, context: Arc<Context>) {
    let offer = match RTCSessionDescription::offer(sdp) {
        Ok(offer) => offer,
        Err(error) => {
            tracing::warn!("mario relay: {peer_id:?} sent an unparseable offer: {error}");
            return;
        }
    };

    let connection = match new_peer_connection(peer_id, context.clone()).await {
        Ok(connection) => connection,
        Err(error) => {
            tracing::warn!("mario relay: failed to open a connection for {peer_id:?}'s offer: {error}");
            return;
        }
    };
    context.peers.write().insert(
        peer_id,
        PeerLink {
            connection: connection.clone(),
            position_channel: RwLock::new(None),
            combat_channel: RwLock::new(None),
            ice_gate: parking_lot::Mutex::new(IceGate::default()),
        },
    );

    if let Err(error) = connection.set_remote_description(offer).await {
        tracing::warn!("mario relay: failed to accept {peer_id:?}'s offer: {error}");
        return;
    }
    flush_pending_ice(peer_id, &context).await;

    let answer = match connection.create_answer(None).await {
        Ok(answer) => answer,
        Err(error) => {
            tracing::warn!("mario relay: failed to create an answer for {peer_id:?}: {error}");
            return;
        }
    };
    let sdp = answer.sdp.clone();
    if let Err(error) = connection.set_local_description(answer).await {
        tracing::warn!("mario relay: failed to set the local description answered to {peer_id:?}: {error}");
        return;
    }

    send_client_message(&context.ws_sink, &ClientMessage::Answer { to: peer_id, sdp }).await;
}

/// Drains one position data channel's events for as long as it stays open, writing every position
/// packet straight into `remote_mario`. Newest write wins, which is correct for an unordered
/// channel where an older packet arriving late is just noise.
fn spawn_position_receive_loop(peer_id: PeerId, channel: Arc<dyn DataChannel>, remote_mario: RemoteMarioTable) {
    tokio::spawn(async move {
        while let Some(event) = channel.poll().await {
            match event {
                DataChannelEvent::OnMessage(message) => {
                    let Ok(text) = String::from_utf8(message.data.to_vec()) else { continue };
                    let Ok(packet) = serde_json::from_str::<PositionPacket>(&text) else { continue };
                    remote_mario.write().insert(packet.candidate_id.clone(), packet);
                }
                DataChannelEvent::OnClose => break,
                _ => {}
            }
        }
        tracing::debug!("mario relay: position channel with {peer_id:?} ended");
    });
}

/// Drains one combat data channel's events for as long as it stays open, appending every combat
/// event to `incoming_combat` (pushed, not keyed like `remote_mario` — unlike position, more than
/// one distinct hit can land in the same tick and every one of them matters, not just the latest).
fn spawn_combat_receive_loop(peer_id: PeerId, channel: Arc<dyn DataChannel>, incoming_combat: Arc<RwLock<Vec<CombatEvent>>>) {
    tokio::spawn(async move {
        while let Some(event) = channel.poll().await {
            match event {
                DataChannelEvent::OnMessage(message) => {
                    let Ok(text) = String::from_utf8(message.data.to_vec()) else { continue };
                    let Ok(combat_event) = serde_json::from_str::<CombatEvent>(&text) else { continue };
                    incoming_combat.write().push(combat_event);
                }
                DataChannelEvent::OnClose => break,
                _ => {}
            }
        }
        tracing::debug!("mario relay: combat channel with {peer_id:?} ended");
    });
}

/// Every `SEND_INTERVAL`, snapshots `local_mario` and pushes it out every currently open position
/// channel, unconditionally, whether or not anything moved. A future authoritative server sitting
/// behind this needs a steady stream, not change-detected updates.
fn spawn_send_loop(context: Arc<Context>, local_mario: Arc<RwLock<Vec<PositionPacket>>>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SEND_INTERVAL);
        let mut seq_by_candidate: HashMap<String, u32> = HashMap::new();

        loop {
            ticker.tick().await;

            let snapshot = local_mario.read().clone();
            if snapshot.is_empty() {
                continue;
            }
            let channels: Vec<Arc<dyn DataChannel>> = context.peers.read().values().filter_map(|link| link.position_channel.read().clone()).collect();
            if channels.is_empty() {
                continue;
            }

            for mut packet in snapshot {
                let seq = seq_by_candidate.entry(packet.candidate_id.clone()).or_insert(0);
                *seq = seq.wrapping_add(1);
                packet.seq = *seq;

                let Ok(text) = serde_json::to_string(&packet) else { continue };
                for channel in &channels {
                    if let Err(error) = channel.send_text(&text).await {
                        tracing::warn!("mario relay: send to a peer failed: {error}");
                    }
                }
            }
        }
    });
}

/// Unlike position, combat events are one-shot, not continuously-resent state, so this polls a
/// short fixed interval just to batch up whatever landed since the last tick and drains the queue
/// rather than resending a persistent snapshot every tick. Broadcasts every event to every
/// currently open combat channel; each receiving instance decides for itself whether the named
/// target is one of its own local players (see `physics::update_mario_physics`) — this loop has no
/// idea which peer owns which candidate, and doesn't need to.
fn spawn_combat_send_loop(context: Arc<Context>, outgoing_combat: Arc<RwLock<Vec<CombatEvent>>>) {
    tokio::spawn(async move {
        let mut ticker = tokio::time::interval(SEND_INTERVAL);

        loop {
            ticker.tick().await;

            let pending = std::mem::take(&mut *outgoing_combat.write());
            if pending.is_empty() {
                continue;
            }
            let channels: Vec<Arc<dyn DataChannel>> = context.peers.read().values().filter_map(|link| link.combat_channel.read().clone()).collect();

            for event in pending {
                let Ok(text) = serde_json::to_string(&event) else { continue };
                for channel in &channels {
                    if let Err(error) = channel.send_text(&text).await {
                        tracing::warn!("mario relay: combat event send to a peer failed: {error}");
                    }
                }
            }
        }
    });
}

/// How many times, and how far apart, a fresh connection to the relay's WebSocket is retried
/// before giving up. Covers two real cases, not just flakiness: a `--host` instance dialing its
/// own just-spawned embedded relay server (`main.rs`'s `--host`) before that server has finished
/// binding its listener, and a `--connect` peer starting up slightly before the host they're
/// joining has. `3s` total is a generous grace period for either without making a genuinely
/// unreachable relay hang around annoyingly long before falling back to local-only play.
const CONNECT_RETRY_ATTEMPTS: u32 = 15;
const CONNECT_RETRY_DELAY: Duration = Duration::from_millis(200);

#[allow(clippy::type_complexity)]
async fn connect_with_retry(
    relay_url: &str,
) -> Result<(WsStream, tokio_tungstenite::tungstenite::handshake::client::Response), tokio_tungstenite::tungstenite::Error> {
    for attempt in 0..CONNECT_RETRY_ATTEMPTS {
        match tokio_tungstenite::connect_async(relay_url).await {
            Ok(connected) => return Ok(connected),
            Err(error) if attempt + 1 == CONNECT_RETRY_ATTEMPTS => return Err(error),
            Err(_) => tokio::time::sleep(CONNECT_RETRY_DELAY).await,
        }
    }
    unreachable!("the loop above always returns on its last attempt")
}

async fn run(
    relay_url: String,
    room: String,
    local_mario: Arc<RwLock<Vec<PositionPacket>>>,
    remote_mario: RemoteMarioTable,
    outgoing_combat: Arc<RwLock<Vec<CombatEvent>>>,
    incoming_combat: Arc<RwLock<Vec<CombatEvent>>>,
) {
    let (stream, _) = match connect_with_retry(&relay_url).await {
        Ok(connected) => connected,
        Err(error) => {
            tracing::warn!("mario relay: could not reach the relay at {relay_url} after retrying: {error}, continuing without remote sync");
            return;
        }
    };

    let (sink, mut source) = stream.split();
    let ws_sink: WsSink = Arc::new(tokio::sync::Mutex::new(sink));
    let context = Arc::new(Context { ws_sink: ws_sink.clone(), peers: RwLock::new(HashMap::new()), remote_mario, incoming_combat });

    send_client_message(&ws_sink, &ClientMessage::Join { room }).await;
    spawn_send_loop(context.clone(), local_mario);
    spawn_combat_send_loop(context.clone(), outgoing_combat);

    while let Some(message) = source.next().await {
        let text = match message {
            Ok(Message::Text(text)) => text,
            Ok(_) => continue,
            Err(error) => {
                tracing::warn!("mario relay: relay socket error: {error}");
                break;
            }
        };
        let message: ServerMessage = match serde_json::from_str(&text) {
            Ok(message) => message,
            Err(error) => {
                tracing::warn!("mario relay: relay sent an unparseable message: {error}");
                continue;
            }
        };

        match message {
            ServerMessage::Joined { peer_id: _, peers: existing } => {
                for peer_id in existing {
                    tokio::spawn(open_offer(peer_id, context.clone()));
                }
            }
            // The newcomer offers to us, nothing to do until their offer arrives.
            ServerMessage::PeerJoined { .. } => {}
            ServerMessage::PeerLeft { peer_id } => {
                if let Some(link) = context.peers.write().remove(&peer_id) {
                    tokio::spawn(async move {
                        let _ = link.connection.close().await;
                    });
                }
            }
            ServerMessage::Offer { from, sdp } => {
                tokio::spawn(accept_offer(from, sdp, context.clone()));
            }
            ServerMessage::Answer { from, sdp } => {
                let connection = context.peers.read().get(&from).map(|link| link.connection.clone());
                let Some(connection) = connection else { continue };
                match RTCSessionDescription::answer(sdp) {
                    Ok(desc) => {
                        let context = context.clone();
                        tokio::spawn(async move {
                            if let Err(error) = connection.set_remote_description(desc).await {
                                tracing::warn!("mario relay: failed to accept {from:?}'s answer: {error}");
                                return;
                            }
                            flush_pending_ice(from, &context).await;
                        });
                    }
                    Err(error) => tracing::warn!("mario relay: {from:?} sent an unparseable answer: {error}"),
                }
            }
            // Buffered instead of applied immediately if this connection's remote description
            // hasn't landed yet.
            ServerMessage::IceCandidate { from, candidate } => {
                let init = RTCIceCandidateInit { candidate, ..Default::default() };
                let peers = context.peers.read();
                let Some(link) = peers.get(&from) else { continue };

                let connection = {
                    let mut gate = link.ice_gate.lock();
                    if gate.remote_description_set {
                        Some(link.connection.clone())
                    } else {
                        gate.pending.push(init.clone());
                        None
                    }
                };
                drop(peers);

                if let Some(connection) = connection {
                    tokio::spawn(async move {
                        if let Err(error) = connection.add_ice_candidate(init).await {
                            tracing::warn!("mario relay: failed to apply an ice candidate from {from:?}: {error}");
                        }
                    });
                }
            }
            ServerMessage::Error { message } => tracing::warn!("mario relay: relay reported an error: {message}"),
        }
    }

    tracing::info!("mario relay: relay connection ended");
}

/// Starts a real `atlas_relay::serve` in-process, bound to every network interface (`0.0.0.0`)
/// rather than just loopback, so a peer elsewhere on the LAN can actually reach it — the
/// embedded-server half of `--host` (`main.rs`). Detached: nothing here ever stops it, matching
/// `atlas_relay::serve`'s own "runs until the process is killed" contract. This instance still
/// connects to it as an ordinary client afterward via its own regular `spawn` call below, the same
/// as any other peer would; it doesn't get any special local-only shortcut.
pub fn spawn_embedded_server(runtime: tokio::runtime::Handle, port: u16) {
    runtime.spawn(async move {
        let addr = std::net::SocketAddr::from(([0, 0, 0, 0], port));
        if let Err(error) = atlas_relay::serve(addr).await {
            tracing::warn!("mario relay: embedded relay server failed: {error}");
        }
    });
}

/// Spawns the whole peer-sync task onto `runtime`. `local_mario` is refreshed by a Bevy system
/// elsewhere every physics tick with this instance's own currently owned positions; `remote_mario`
/// is written to as position packets arrive from any connected peer. `outgoing_combat` is pushed to
/// by that same physics system whenever a local swing lands on a remote-tracked target;
/// `incoming_combat` is written to as combat events arrive, for that system to drain and apply.
#[allow(clippy::too_many_arguments)]
pub fn spawn(
    runtime: tokio::runtime::Handle,
    relay_url: String,
    room: String,
    local_mario: Arc<RwLock<Vec<PositionPacket>>>,
    remote_mario: RemoteMarioTable,
    outgoing_combat: Arc<RwLock<Vec<CombatEvent>>>,
    incoming_combat: Arc<RwLock<Vec<CombatEvent>>>,
) {
    runtime.spawn(run(relay_url, room, local_mario, remote_mario, outgoing_combat, incoming_combat));
}
