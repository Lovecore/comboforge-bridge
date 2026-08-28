#![allow(dead_code)] // the #[path] shims compile modules whose full surface this test does not use
//! Mock poller + real WebSocket server + real client, no hardware: the whole
//! pipeline CI can run on a machine that has never seen a controller.

use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;
use tokio_tungstenite::tungstenite::Message;

use comboforge_bridge_protocol::ServerMessage;

// The binary crate's modules are not importable from an integration test, so
// the test wires the same components through their public seams: it spawns
// the real binary? No -- simpler and more honest: this test builds the exact
// wiring main() uses, via a tiny test harness copied from main's shape.
// To keep ONE source of truth, the harness lives here and is deliberately
// dumb; the components it wires (differ, mock script, server) are the very
// ones the binary uses because they are exercised through the compiled
// binary in the release smoke script.

#[path = "../src/clock.rs"]
mod clock;
#[path = "../src/config.rs"]
mod config;
#[path = "../src/input/mod.rs"]
mod input;
#[path = "../src/server/mod.rs"]
mod server;
#[path = "../src/token.rs"]
mod token;

// config.rs and server/client.rs reference `crate::` paths; shim them.
use clock as crate_clock;

const TOKEN: &str = "TEST-TOKEN-1234-5678-ABCD";

async fn connect(
    port: u16,
    origin: Option<&str>,
) -> Result<
    tokio_tungstenite::WebSocketStream<tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>>,
    tokio_tungstenite::tungstenite::Error,
> {
    let mut request = format!("ws://127.0.0.1:{port}/")
        .into_client_request()
        .unwrap();
    if let Some(origin) = origin {
        request
            .headers_mut()
            .insert("Origin", origin.parse().unwrap());
    }
    tokio_tungstenite::connect_async(request)
        .await
        .map(|(ws, _)| ws)
}

#[tokio::test(flavor = "multi_thread")]
async fn the_whole_pipeline_without_hardware() {
    let config = config::Config {
        token: TOKEN.into(),
        extra_origins: vec![],
    };
    let clock = Arc::new(clock::Clock::start());
    let (edge_tx, _) = tokio::sync::broadcast::channel::<input::Edge>(1024);
    let (snap_tx, snap_rx) = tokio::sync::watch::channel(input::PadSnapshot::empty());

    {
        let edge_tx = edge_tx.clone();
        let clock = Arc::clone(&clock);
        std::thread::spawn(move || input::mock::run(500, clock, edge_tx, snap_tx));
    }
    tokio::spawn(server::run(config, clock, 500, edge_tx, snap_rx));
    tokio::time::sleep(Duration::from_millis(300)).await;

    // Find which of the candidate ports the server actually took.
    let mut port = None;
    for candidate in server::PORTS {
        if connect(candidate, Some("https://comboforge.gg"))
            .await
            .is_ok()
        {
            port = Some(candidate);
            break;
        }
    }
    let port = port.expect("server did not come up on any candidate port");

    // 1. No Origin -> the upgrade itself is refused.
    assert!(
        connect(port, None).await.is_err(),
        "originless connect must fail"
    );

    // 2. Bad token -> in-band error, then close.
    {
        let mut ws = connect(port, Some("https://comboforge.gg")).await.unwrap();
        ws.send(Message::Text(
            r#"{"type":"auth","token":"WRONG","protocol":{"major":1,"minorMin":0,"minorMax":0},"client":"e2e"}"#.to_string(),
        ))
        .await
        .unwrap();
        let mut saw_bad_token = false;
        while let Some(Ok(message)) = ws.next().await {
            if let Message::Text(text) = message {
                if text.contains("badToken") {
                    saw_bad_token = true;
                }
            }
        }
        assert!(saw_bad_token, "wrong token should get the named error");
    }

    // 3. Good token -> hello, edges from the scripted pad, heartbeat with
    //    clients:1, and per-connection seq starting at 1.
    let mut ws = connect(port, Some("https://comboforge.gg")).await.unwrap();
    ws.send(Message::Text(
        format!(r#"{{"type":"auth","token":"{TOKEN}","protocol":{{"major":1,"minorMin":0,"minorMax":0}},"client":"e2e"}}"#),
    ))
    .await
    .unwrap();

    let mut saw_hello = false;
    let mut saw_button_down = false;
    let mut saw_heartbeat_with_me = false;
    let mut last_seq = 0u32;
    let deadline = tokio::time::Instant::now() + Duration::from_secs(6);
    while tokio::time::Instant::now() < deadline {
        let Ok(Some(Ok(Message::Text(text)))) =
            tokio::time::timeout(Duration::from_secs(2), ws.next()).await
        else {
            break;
        };
        let message: ServerMessage =
            serde_json::from_str(&text).expect("server speaks the protocol");
        match message {
            ServerMessage::Hello {
                protocol, devices, ..
            } => {
                assert_eq!(protocol.major, 1);
                assert_eq!(devices.len(), 1, "the mock announces one pad");
                assert_eq!(devices[0].mapping, "standard");
                saw_hello = true;
            }
            ServerMessage::ButtonDown { seq, t, index, .. } => {
                assert!(seq > last_seq, "seq must be monotonic per connection");
                last_seq = seq;
                assert!(t > 0, "edges carry the poll-instant timestamp");
                if index == 0 {
                    saw_button_down = true;
                }
            }
            ServerMessage::ButtonUp { seq, .. }
            | ServerMessage::ButtonValue { seq, .. }
            | ServerMessage::Axis { seq, .. } => {
                assert!(seq > last_seq);
                last_seq = seq;
            }
            ServerMessage::Heartbeat { seq, clients, .. } => {
                assert!(seq > last_seq);
                last_seq = seq;
                if clients >= 1 {
                    saw_heartbeat_with_me = true;
                }
            }
            ServerMessage::Error { .. } => panic!("unexpected error frame"),
        }
        if saw_hello && saw_button_down && saw_heartbeat_with_me {
            break;
        }
    }
    assert!(saw_hello, "no hello");
    assert!(saw_button_down, "the scripted A press never arrived");
    assert!(saw_heartbeat_with_me, "no heartbeat counting this client");
}

// Silence unused-shim warnings: the test only needs a subset of each module.
#[allow(unused)]
fn _shims() {
    let _ = token::generate;
    let _ = crate_clock::Clock::start;
}
