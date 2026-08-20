//! Drives the relay with two real WebSocket clients (`tokio-tungstenite`, not the server's own
//! code) — proves the actual wire protocol works, not just that the server's internal types
//! compile.

use std::time::Duration;

use futures_util::SinkExt;
use futures_util::StreamExt;
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;

/// Every helper here returns `Result` and every test propagates via `?` rather than panicking
/// mid-helper — a failure then reports as one clear error at the actual assertion site instead of
/// an opaque panic several calls deep inside a shared helper.
type TestResult<T = ()> = Result<T, Box<dyn std::error::Error>>;

async fn spawn_relay() -> TestResult<String> {
    let listener = TcpListener::bind("127.0.0.1:0").await?;
    let addr = listener.local_addr()?;
    drop(listener);

    tokio::spawn(atlas_relay::serve(addr));
    // Give the server a moment to actually start listening before the test connects.
    tokio::time::sleep(Duration::from_millis(50)).await;

    Ok(format!("ws://{addr}/ws"))
}

async fn recv_json(socket: &mut (impl StreamExt<Item = Result<Message, tokio_tungstenite::tungstenite::Error>> + Unpin)) -> TestResult<serde_json::Value> {
    let message = tokio::time::timeout(Duration::from_secs(2), socket.next())
        .await
        .map_err(|_| "timed out waiting for a message")?
        .ok_or("socket closed while waiting for a message")??;

    let Message::Text(text) = message else {
        return Err(format!("expected a text frame, got {message:?}").into());
    };
    Ok(serde_json::from_str(&text)?)
}

#[tokio::test]
async fn two_peers_relay_a_handshake() -> TestResult {
    let url = spawn_relay().await?;

    let (mut alice, _) = tokio_tungstenite::connect_async(&url).await?;
    let (mut bob, _) = tokio_tungstenite::connect_async(&url).await?;

    alice.send(Message::Text(r#"{"type":"join","room":"test-room"}"#.into())).await?;
    let alice_joined = recv_json(&mut alice).await?;
    assert_eq!(alice_joined["type"], "joined");
    let alice_id = alice_joined["peer_id"].as_str().ok_or("alice has no peer_id")?.to_string();
    assert_eq!(alice_joined["peers"].as_array().ok_or("peers isn't an array")?.len(), 0, "alice is first in the room, no peers yet");

    bob.send(Message::Text(r#"{"type":"join","room":"test-room"}"#.into())).await?;
    let bob_joined = recv_json(&mut bob).await?;
    assert_eq!(bob_joined["type"], "joined");
    let bob_id = bob_joined["peer_id"].as_str().ok_or("bob has no peer_id")?.to_string();
    assert_eq!(bob_joined["peers"].as_array().ok_or("peers isn't an array")?[0], alice_id, "bob sees alice already in the room");

    // Alice gets told bob joined.
    let alice_notified = recv_json(&mut alice).await?;
    assert_eq!(alice_notified["type"], "peer-joined");
    assert_eq!(alice_notified["peer_id"], bob_id);

    // Alice sends bob a real-shaped SDP offer, addressed by bob's peer ID.
    let offer = format!(r#"{{"type":"offer","to":"{bob_id}","sdp":"v=0 fake-sdp-offer"}}"#);
    alice.send(Message::Text(offer.into())).await?;

    let bob_received_offer = recv_json(&mut bob).await?;
    assert_eq!(bob_received_offer["type"], "offer");
    assert_eq!(bob_received_offer["from"], alice_id, "the relay stamps the real sender, not whatever alice claimed");
    assert_eq!(bob_received_offer["sdp"], "v=0 fake-sdp-offer");

    // Bob answers back.
    let answer = format!(r#"{{"type":"answer","to":"{alice_id}","sdp":"v=0 fake-sdp-answer"}}"#);
    bob.send(Message::Text(answer.into())).await?;

    let alice_received_answer = recv_json(&mut alice).await?;
    assert_eq!(alice_received_answer["type"], "answer");
    assert_eq!(alice_received_answer["from"], bob_id);
    assert_eq!(alice_received_answer["sdp"], "v=0 fake-sdp-answer");

    // And an ICE candidate, same relay path.
    let candidate = format!(r#"{{"type":"ice-candidate","to":"{alice_id}","candidate":"fake-candidate"}}"#);
    bob.send(Message::Text(candidate.into())).await?;

    let alice_received_candidate = recv_json(&mut alice).await?;
    assert_eq!(alice_received_candidate["type"], "ice-candidate");
    assert_eq!(alice_received_candidate["from"], bob_id);
    assert_eq!(alice_received_candidate["candidate"], "fake-candidate");

    // Bob disconnects, alice should be told he left.
    bob.close(None).await.ok();
    let alice_notified_leave = recv_json(&mut alice).await?;
    assert_eq!(alice_notified_leave["type"], "peer-left");
    assert_eq!(alice_notified_leave["peer_id"], bob_id);

    Ok(())
}
