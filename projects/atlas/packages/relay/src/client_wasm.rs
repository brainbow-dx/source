//! A generic browser-side WebRTC-over-`atlas-relay` peer client, using `web-sys`'s
//! `RtcPeerConnection`/`RtcDataChannel`/`WebSocket` bindings (browser-native WebRTC, since
//! `webrtc-rs` -- what every native peer, e.g. `escher`'s `mario` example, uses -- doesn't target
//! wasm32). Speaks the exact same `crate::protocol::{ClientMessage, ServerMessage}` JSON the
//! native signaling path (this crate's own `lib.rs`/`serve`) does, reusing those types directly
//! rather than a second hand-rolled copy of the wire shape, so a browser peer negotiates correctly
//! against real native peers already in a room.
//!
//! Deliberately generic: this module knows nothing about any particular game's packet shapes
//! (positions, combat events, ...) -- it only manages the relay handshake, per-peer
//! `RtcPeerConnection`s, and named data channels carrying opaque text. A consumer (like
//! `mario-wasm`) supplies the channel labels/reliability it wants at construction and sends/polls
//! plain strings by label; it's the consumer's job to know what those strings mean.
//!
//! `#[cfg(target_arch = "wasm32")]`-only (see this crate's own `lib.rs`) -- lives in the same
//! crate as the native signaling server rather than a separate one, so a wasm32 consumer depends
//! on plain `atlas-relay` exactly like a native one does, with no extra crate in the graph. This
//! crate's `axum`/`tokio` server dependencies are themselves target-gated out for wasm32 in
//! `Cargo.toml`, so this module doesn't drag them in.
//!
//! Negotiation follows the same one rule the native `lib.rs`/`escher`'s own `relay.rs` documents,
//! to avoid a two-sided offer race: whoever the relay's `Joined` message names as already-present
//! peers gets offered a connection by this (newcomer) client; whoever joins after this client is
//! already in the room instead sends this client an offer.
//!
//! No ICE servers are configured -- correct for same-host or LAN peers (host candidates alone are
//! enough), matching the native side's own `new_peer_connection` doc comment. Peers on separate
//! networks with no STUN/TURN reachable between them won't connect; out of scope for this build.

use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

use crate::protocol::ClientMessage;
use crate::protocol::PeerId;
use crate::protocol::ServerMessage;

use wasm_bindgen::prelude::*;
use wasm_bindgen::JsCast;
use wasm_bindgen_futures::JsFuture;

use web_sys::MessageEvent;
use web_sys::RtcConfiguration;
use web_sys::RtcDataChannel;
use web_sys::RtcDataChannelEvent;
use web_sys::RtcDataChannelInit;
use web_sys::RtcIceCandidateInit;
use web_sys::RtcPeerConnection;
use web_sys::RtcPeerConnectionIceEvent;
use web_sys::RtcSessionDescriptionInit;
use web_sys::WebSocket;

/// One data channel this client should open (as the offerer) or recognize (as the answerer),
/// supplied by the consumer at construction as a small JSON array -- simpler than marshaling a
/// richer JS type across the `wasm-bindgen` boundary for what's really just three plain fields.
#[derive(Debug, Clone, serde::Deserialize)]
struct ChannelSpec {
    label: String,
    ordered: bool,
    max_retransmits: Option<u16>,
}

/// Buffers ICE candidates that arrive before this peer's remote description is set -- same
/// reasoning and shape as the native `escher`/`relay.rs`'s own `IceGate`: candidates arrive as
/// independent relay messages with no ordering guarantee against when the remote description
/// actually lands.
#[derive(Default)]
struct IceGate {
    remote_description_set: bool,
    pending: Vec<RtcIceCandidateInit>,
}

struct PeerLink {
    connection: RtcPeerConnection,
    channels: HashMap<String, RtcDataChannel>,
    ice_gate: IceGate,
}

struct Inner {
    ws: Option<WebSocket>,
    room: String,
    channel_specs: Vec<ChannelSpec>,
    peers: HashMap<uuid::Uuid, PeerLink>,
    /// Received text messages, queued by data-channel label until `poll_messages` drains them.
    inbox: HashMap<String, Vec<String>>,
}

/// One browser tab's whole connection to a relay room. Construct once; `send`/`poll_messages`
/// from the page's own per-frame loop the same way `mario-wasm`'s `Game::tick` already reads
/// keyboard/gamepad state each frame.
#[wasm_bindgen]
pub struct RelayClient {
    inner: Rc<RefCell<Inner>>,
}

#[wasm_bindgen]
impl RelayClient {
    /// `channels_json` is a JSON array of `{"label": string, "ordered": bool,
    /// "maxRetransmits": number|null}`, one entry per data channel this client should open with
    /// each peer it offers to (an answering peer accepts whatever channels the offerer opens,
    /// keyed by label, regardless of what's passed here -- this list only matters for the
    /// offering side's own `create_data_channel` calls).
    #[wasm_bindgen(constructor)]
    pub fn new(ws_url: String, room: String, channels_json: String) -> Result<RelayClient, JsValue> {
        #[derive(serde::Deserialize)]
        struct RawSpec {
            label: String,
            ordered: bool,
            #[serde(rename = "maxRetransmits")]
            max_retransmits: Option<u16>,
        }
        let raw: Vec<RawSpec> = serde_json::from_str(&channels_json).map_err(|error| JsValue::from_str(&error.to_string()))?;
        let channel_specs =
            raw.into_iter().map(|spec| ChannelSpec { label: spec.label, ordered: spec.ordered, max_retransmits: spec.max_retransmits }).collect();

        let inner = Rc::new(RefCell::new(Inner { ws: None, room, channel_specs, peers: HashMap::new(), inbox: HashMap::new() }));
        connect(inner.clone(), ws_url)?;
        Ok(RelayClient { inner })
    }

    /// Sends `text` to every currently open channel with this `label`, across every connected
    /// peer. Silently does nothing for a peer whose channel with that label isn't open yet
    /// (matches the native side's own "networking failures are logged and swallowed, local play
    /// keeps working" posture) -- errors are logged via `console.warn`, not surfaced to the caller.
    pub fn send(&self, label: &str, text: &str) {
        let inner = self.inner.borrow();
        for peer in inner.peers.values() {
            if let Some(channel) = peer.channels.get(label) {
                if channel_is_open(channel) {
                    if let Err(error) = channel.send_with_str(text) {
                        web_sys::console::warn_1(&format!("atlas-relay client_wasm: send on {label:?} failed: {error:?}").into());
                    }
                }
            }
        }
    }

    /// Drains and returns every text message received on `label` since the last call, oldest
    /// first, as a JS array of strings.
    #[wasm_bindgen(js_name = pollMessages)]
    pub fn poll_messages(&self, label: &str) -> Vec<String> {
        self.inner.borrow_mut().inbox.get_mut(label).map(std::mem::take).unwrap_or_default()
    }

    /// How many peers currently have at least one open data channel -- a simple, honest "am I
    /// actually connected to anyone" signal for the page to show, distinct from "the relay
    /// WebSocket itself is open" (which says nothing about whether any peer-to-peer handshake
    /// actually completed).
    #[wasm_bindgen(js_name = connectedPeerCount)]
    pub fn connected_peer_count(&self) -> usize {
        self.inner.borrow().peers.values().filter(|peer| peer.channels.values().any(channel_is_open)).count()
    }
}

/// Whether `channel`'s `readyState` is `"open"` -- read as a raw JS string property
/// (`Reflect::get`), not via `RtcDataChannel::ready_state()`'s typed `RtcDataChannelState` return.
/// Same reasoning as `session_description`'s own doc comment: any `web-sys` "string enum" this
/// module actually references trips `wasm-bindgen-cli` 0.2.126's "duplicate string enums" bug, so
/// every one of them is read/written as a plain string instead, never named as a Rust type.
fn channel_is_open(channel: &RtcDataChannel) -> bool {
    js_sys::Reflect::get(channel, &JsValue::from_str("readyState")).ok().and_then(|value| value.as_string()).as_deref() == Some("open")
}

fn connect(inner: Rc<RefCell<Inner>>, ws_url: String) -> Result<(), JsValue> {
    let ws = WebSocket::new(&ws_url)?;

    {
        let inner = inner.clone();
        let onopen = Closure::<dyn FnMut()>::new(move || {
            let room = inner.borrow().room.clone();
            send_client_message(&inner, &ClientMessage::Join { room });
        });
        ws.set_onopen(Some(onopen.as_ref().unchecked_ref()));
        onopen.forget();
    }

    {
        let inner = inner.clone();
        let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
            let Some(text) = event.data().as_string() else { return };
            let message: ServerMessage = match serde_json::from_str(&text) {
                Ok(message) => message,
                Err(error) => {
                    web_sys::console::warn_1(&format!("atlas-relay client_wasm: unparseable relay message: {error}").into());
                    return;
                }
            };
            handle_server_message(inner.clone(), message);
        });
        ws.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
        onmessage.forget();
    }

    inner.borrow_mut().ws = Some(ws);
    Ok(())
}

fn send_client_message(inner: &Rc<RefCell<Inner>>, message: &ClientMessage) {
    let Ok(text) = serde_json::to_string(message) else { return };
    if let Some(ws) = inner.borrow().ws.as_ref() {
        if let Err(error) = ws.send_with_str(&text) {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to send to the relay: {error:?}").into());
        }
    }
}

fn handle_server_message(inner: Rc<RefCell<Inner>>, message: ServerMessage) {
    match message {
        ServerMessage::Joined { peer_id: _, peers: existing } => {
            for peer_id in existing {
                wasm_bindgen_futures::spawn_local(open_offer(inner.clone(), peer_id));
            }
        }
        // The newcomer offers to us, nothing to do until their offer arrives -- same as native.
        ServerMessage::PeerJoined { .. } => {}
        ServerMessage::PeerLeft { peer_id } => {
            if let Some(link) = inner.borrow_mut().peers.remove(&peer_id.0) {
                let _ = link.connection.close();
            }
        }
        ServerMessage::Offer { from, sdp } => {
            wasm_bindgen_futures::spawn_local(accept_offer(inner.clone(), from, sdp));
        }
        ServerMessage::Answer { from, sdp } => {
            wasm_bindgen_futures::spawn_local(accept_answer(inner.clone(), from, sdp));
        }
        ServerMessage::IceCandidate { from, candidate } => {
            wasm_bindgen_futures::spawn_local(apply_or_buffer_ice(inner.clone(), from, candidate));
        }
        ServerMessage::Error { message } => {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: relay reported an error: {message}").into());
        }
    }
}

fn new_peer_connection() -> Result<RtcPeerConnection, JsValue> {
    // No ICE servers -- see this module's own doc comment. `RtcConfiguration::default()` already
    // carries an empty `iceServers` list.
    let configuration = RtcConfiguration::new();
    RtcPeerConnection::new_with_configuration(&configuration)
}

/// Wires a data channel's `onmessage` to push into `inner`'s inbox under its own label, and (for
/// the offering side, which creates channels before the connection object is registered in
/// `inner.peers`) inserts it into `channels` directly so the caller can store it on the `PeerLink`
/// once that's created. Shared by both the offering (`create_data_channel`) and answering
/// (`ondatachannel`) paths so the message-routing logic isn't duplicated.
fn wire_channel(inner: Rc<RefCell<Inner>>, channel: &RtcDataChannel) {
    let label = channel.label();
    let onmessage_label = label.clone();
    let onmessage = Closure::<dyn FnMut(MessageEvent)>::new(move |event: MessageEvent| {
        let Some(text) = event.data().as_string() else { return };
        inner.borrow_mut().inbox.entry(onmessage_label.clone()).or_default().push(text);
    });
    channel.set_onmessage(Some(onmessage.as_ref().unchecked_ref()));
    onmessage.forget();
}

fn wire_ice_forwarding(inner: Rc<RefCell<Inner>>, connection: &RtcPeerConnection, peer_id: PeerId) {
    let onicecandidate = Closure::<dyn FnMut(RtcPeerConnectionIceEvent)>::new(move |event: RtcPeerConnectionIceEvent| {
        let Some(candidate) = event.candidate() else { return };
        send_client_message(&inner, &ClientMessage::IceCandidate { to: peer_id, candidate: candidate.candidate() });
    });
    connection.set_onicecandidate(Some(onicecandidate.as_ref().unchecked_ref()));
    onicecandidate.forget();
}

fn data_channel_init(spec: &ChannelSpec) -> RtcDataChannelInit {
    let init = RtcDataChannelInit::new();
    init.set_ordered(spec.ordered);
    if let Some(max_retransmits) = spec.max_retransmits {
        init.set_max_retransmits(max_retransmits);
    }
    init
}

/// Builds an `RtcSessionDescriptionInit`-shaped object for `sdp_type` (`"offer"`/`"answer"`, the
/// exact wire strings `RTCSessionDescription` expects) and `sdp`. Deliberately built as a plain JS
/// object with string properties (`Reflect::set`), then cast, rather than via `web_sys::
/// RtcSessionDescriptionInit::new(RtcSdpType::...)` -- a real, reproduced `wasm-bindgen-cli`
/// 0.2.126 bug: using `web_sys::RtcSdpType` (a generated "string enum") at all in this module, with
/// more than one of its variants referenced anywhere in the binary, makes the CLI's post-
/// processing pass see its generated aux type as registered twice and fail outright with
/// "duplicate string enums" -- confirmed independent of call-site count, inlining, and
/// optimization level (all tried and ruled out). Since `RTCSessionDescription`'s wire format is
/// just `{type, sdp}` with `type` a plain string, building it without ever naming `RtcSdpType` in
/// Rust code sidesteps the bug entirely rather than working around it.
fn session_description(sdp_type: &str, sdp: &str) -> RtcSessionDescriptionInit {
    let object = js_sys::Object::new();
    let _ = js_sys::Reflect::set(&object, &JsValue::from_str("type"), &JsValue::from_str(sdp_type));
    let _ = js_sys::Reflect::set(&object, &JsValue::from_str("sdp"), &JsValue::from_str(sdp));
    object.unchecked_into::<RtcSessionDescriptionInit>()
}

/// This client is the newcomer to `peer_id`'s room membership, so it offers, creating every
/// configured data channel up front -- mirrors the native `escher`/`relay.rs::open_offer`.
async fn open_offer(inner: Rc<RefCell<Inner>>, peer_id: PeerId) {
    let connection = match new_peer_connection() {
        Ok(connection) => connection,
        Err(error) => {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to open a connection to {peer_id:?}: {error:?}").into());
            return;
        }
    };
    wire_ice_forwarding(inner.clone(), &connection, peer_id);

    let specs = inner.borrow().channel_specs.clone();
    let mut channels = HashMap::new();
    for spec in &specs {
        let init = data_channel_init(spec);
        let channel = connection.create_data_channel_with_data_channel_dict(&spec.label, &init);
        wire_channel(inner.clone(), &channel);
        channels.insert(spec.label.clone(), channel);
    }

    inner.borrow_mut().peers.insert(peer_id.0, PeerLink { connection: connection.clone(), channels, ice_gate: IceGate::default() });

    let offer = match JsFuture::from(connection.create_offer()).await {
        Ok(offer) => offer,
        Err(error) => {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to create an offer for {peer_id:?}: {error:?}").into());
            return;
        }
    };
    let Some(sdp) = js_sys::Reflect::get(&offer, &JsValue::from_str("sdp")).ok().and_then(|value| value.as_string()) else { return };

    let description = session_description("offer", &sdp);
    if let Err(error) = JsFuture::from(connection.set_local_description(&description)).await {
        web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to set local description offered to {peer_id:?}: {error:?}").into());
        return;
    }

    send_client_message(&inner, &ClientMessage::Offer { to: peer_id, sdp });
}

/// `peer_id` is already in the room and offered us a connection, so this client answers. Incoming
/// data channels arrive later, asynchronously, via `ondatachannel` -- the offerer's
/// `create_data_channel` calls are what open them.
async fn accept_offer(inner: Rc<RefCell<Inner>>, peer_id: PeerId, sdp: String) {
    let connection = match new_peer_connection() {
        Ok(connection) => connection,
        Err(error) => {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to open a connection for {peer_id:?}'s offer: {error:?}").into());
            return;
        }
    };
    wire_ice_forwarding(inner.clone(), &connection, peer_id);

    {
        let inner = inner.clone();
        let ondatachannel = Closure::<dyn FnMut(RtcDataChannelEvent)>::new(move |event: RtcDataChannelEvent| {
            let channel = event.channel();
            wire_channel(inner.clone(), &channel);
            let label = channel.label();
            if let Some(link) = inner.borrow_mut().peers.get_mut(&peer_id.0) {
                link.channels.insert(label, channel);
            }
        });
        connection.set_ondatachannel(Some(ondatachannel.as_ref().unchecked_ref()));
        ondatachannel.forget();
    }

    inner.borrow_mut().peers.insert(peer_id.0, PeerLink { connection: connection.clone(), channels: HashMap::new(), ice_gate: IceGate::default() });

    let description = session_description("offer", &sdp);
    if let Err(error) = JsFuture::from(connection.set_remote_description(&description)).await {
        web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to accept {peer_id:?}'s offer: {error:?}").into());
        return;
    }
    flush_pending_ice(&inner, peer_id).await;

    let answer = match JsFuture::from(connection.create_answer()).await {
        Ok(answer) => answer,
        Err(error) => {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to create an answer for {peer_id:?}: {error:?}").into());
            return;
        }
    };
    let Some(answer_sdp) = js_sys::Reflect::get(&answer, &JsValue::from_str("sdp")).ok().and_then(|value| value.as_string()) else { return };

    let answer_description = session_description("answer", &answer_sdp);
    if let Err(error) = JsFuture::from(connection.set_local_description(&answer_description)).await {
        web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to set local description answered to {peer_id:?}: {error:?}").into());
        return;
    }

    send_client_message(&inner, &ClientMessage::Answer { to: peer_id, sdp: answer_sdp });
}

async fn accept_answer(inner: Rc<RefCell<Inner>>, peer_id: PeerId, sdp: String) {
    let connection = { inner.borrow().peers.get(&peer_id.0).map(|link| link.connection.clone()) };
    let Some(connection) = connection else { return };

    let description = session_description("answer", &sdp);
    if let Err(error) = JsFuture::from(connection.set_remote_description(&description)).await {
        web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to accept {peer_id:?}'s answer: {error:?}").into());
        return;
    }
    flush_pending_ice(&inner, peer_id).await;
}

/// Marks `peer_id`'s remote description as set and applies every ICE candidate buffered before
/// this point -- same shape as native `escher`/`relay.rs::flush_pending_ice`.
async fn flush_pending_ice(inner: &Rc<RefCell<Inner>>, peer_id: PeerId) {
    let (connection, pending) = {
        let mut inner = inner.borrow_mut();
        let Some(link) = inner.peers.get_mut(&peer_id.0) else { return };
        link.ice_gate.remote_description_set = true;
        (link.connection.clone(), std::mem::take(&mut link.ice_gate.pending))
    };

    for candidate in pending {
        if let Err(error) = JsFuture::from(connection.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&candidate))).await {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to apply a buffered ice candidate from {peer_id:?}: {error:?}").into());
        }
    }
}

/// Applies an incoming ICE candidate immediately if `peer_id`'s remote description is already
/// set, or buffers it otherwise -- candidates can arrive over the relay socket before the
/// offer/answer exchange that creates something to attach them to.
async fn apply_or_buffer_ice(inner: Rc<RefCell<Inner>>, peer_id: PeerId, candidate: String) {
    let init = RtcIceCandidateInit::new(&candidate);
    // The relay's wire protocol only carries the bare candidate string (`ClientMessage::
    // IceCandidate`/`ServerMessage::IceCandidate` have no `sdpMid`/`sdpMLineIndex` fields) --
    // native `webrtc-rs` peers tolerate that, but the browser's own `addIceCandidate` does not:
    // real, reproduced bug, `RtcIceCandidateInit::new` alone throws "Candidate missing values for
    // both sdpMid and sdpMLineIndex" every time, which silently killed every ICE candidate and
    // meant no data channel could ever open. There's exactly one m-line in this SDP -- the
    // datachannel-only `m=application ... webrtc-datachannel` offer both sides always send (see
    // `new_peer_connection`'s own data-channel-only setup) -- so `mid "0"`/index `0` is always
    // correct, not a guess specific to any one negotiation.
    init.set_sdp_mid(Some("0"));
    init.set_sdp_m_line_index(Some(0));

    let connection = {
        let mut inner = inner.borrow_mut();
        let Some(link) = inner.peers.get_mut(&peer_id.0) else { return };
        if link.ice_gate.remote_description_set {
            Some(link.connection.clone())
        } else {
            link.ice_gate.pending.push(init.clone());
            None
        }
    };

    if let Some(connection) = connection {
        if let Err(error) = JsFuture::from(connection.add_ice_candidate_with_opt_rtc_ice_candidate_init(Some(&init))).await {
            web_sys::console::warn_1(&format!("atlas-relay client_wasm: failed to apply an ice candidate from {peer_id:?}: {error:?}").into());
        }
    }
}
