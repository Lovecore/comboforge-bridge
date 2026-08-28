//! One connection: origin gate at upgrade, auth within 3s, then hello and
//! the event stream. `seq` is PER-CONNECTION starting at 1, so every client
//! sees an unbroken sequence regardless of when it joined; a gap means WE
//! dropped events for THIS client under backpressure, and the next
//! heartbeat's snapshot is the resync.

use std::sync::atomic::Ordering;
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpStream;
use tokio_tungstenite::tungstenite::handshake::server::{ErrorResponse, Request, Response};
use tokio_tungstenite::tungstenite::Message;

use comboforge_bridge_protocol::{ClockInfo, HelperInfo, ProtocolVersion, ServerMessage};

use super::handshake::{check_auth, origin_allowed, AuthOutcome};
use super::Shared;
use crate::input::{Edge, EdgeKind, PadSnapshot};

const AUTH_TIMEOUT: Duration = Duration::from_secs(3);
const CLOSE_BAD_AUTH: u16 = 4401;
const CLOSE_TIMEOUT: u16 = 4408;
const CLOSE_PROTOCOL: u16 = 4426;

pub async fn serve(
    stream: TcpStream,
    shared: Arc<Shared>,
    mut edges: tokio::sync::broadcast::Receiver<Edge>,
    snapshots: tokio::sync::watch::Receiver<PadSnapshot>,
) {
    let extra = shared.config.extra_origins.clone();
    let mut origin: Option<String> = None;
    let callback = |req: &Request, resp: Response| {
        let header = req
            .headers()
            .get("Origin")
            .and_then(|v| v.to_str().ok())
            .map(|s| s.to_string());
        if origin_allowed(header.as_deref(), &extra) {
            origin = header;
            Ok(resp)
        } else {
            let shown = header.as_deref().unwrap_or("(no Origin)");
            println!("  connection from {shown} -- ORIGIN REJECTED (403)");
            // Default-deny is the security model; a preview/staging deploy of
            // ComboForge is the one legitimate caller that lands here, so the
            // rejection teaches the fix instead of just refusing.
            if shown.contains("vercel.app") || shown.contains("comboforge") {
                println!(
                    "    to allow it: add \"{shown}\" to \"extraOrigins\" in your config.json and restart"
                );
            }
            let mut response = ErrorResponse::new(Some("origin not allowed".into()));
            *response.status_mut() = tokio_tungstenite::tungstenite::http::StatusCode::FORBIDDEN;
            Err(response)
        }
    };

    let Ok(ws) = tokio_tungstenite::accept_hdr_async(stream, callback).await else {
        return;
    };
    let origin = origin.unwrap_or_default();
    let (mut sink, mut source) = ws.split();

    // --- auth, or nothing else ---
    let first = tokio::time::timeout(AUTH_TIMEOUT, source.next()).await;
    let raw = match first {
        Ok(Some(Ok(Message::Text(text)))) => text,
        Ok(_) => {
            let _ = close(&mut sink, CLOSE_BAD_AUTH).await;
            return;
        }
        Err(_) => {
            let _ = close(&mut sink, CLOSE_TIMEOUT).await;
            return;
        }
    };
    match check_auth(&raw, &shared.config.token) {
        AuthOutcome::Accept => {}
        AuthOutcome::BadToken => {
            println!("  connection from {origin} -- token REJECTED");
            let _ = send(
                &mut sink,
                &ServerMessage::Error {
                    code: "badToken".into(),
                    message: "Pairing token did not match".into(),
                },
            )
            .await;
            let _ = close(&mut sink, CLOSE_BAD_AUTH).await;
            return;
        }
        AuthOutcome::ProtocolMismatch { server_major } => {
            let _ = send(
                &mut sink,
                &ServerMessage::Error {
                    code: "protocolMismatch".into(),
                    message: format!("server speaks protocol major {server_major}"),
                },
            )
            .await;
            let _ = close(&mut sink, CLOSE_PROTOCOL).await;
            return;
        }
        AuthOutcome::NotAuth => {
            let _ = close(&mut sink, CLOSE_BAD_AUTH).await;
            return;
        }
    }

    let count = shared.clients.fetch_add(1, Ordering::SeqCst) + 1;
    println!(
        "  connection from {origin} -- token ok -- client {count} of {}",
        super::MAX_CLIENTS
    );
    println!("  Clients connected: {count}");

    let mut seq: u32 = 0;
    let mut next_seq = || {
        seq = seq.wrapping_add(1);
        seq
    };

    // --- hello ---
    let snapshot = snapshots.borrow().clone();
    let hello = ServerMessage::Hello {
        protocol: ProtocolVersion::CURRENT,
        helper: HelperInfo {
            name: "comboforge-bridge".into(),
            version: env!("CARGO_PKG_VERSION").into(),
        },
        clock: ClockInfo {
            t_us: shared.clock.t_us(),
            wall_ms: shared.clock.wall_ms(),
            poll_hz: shared.poll_hz,
        },
        devices: snapshot.to_protocol_devices(),
        state: snapshot.to_protocol_state(),
    };
    if send(&mut sink, &hello).await.is_err() {
        finish(&shared);
        return;
    }

    // --- the stream ---
    let mut heartbeat = tokio::time::interval(Duration::from_secs(1));
    heartbeat.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);
    loop {
        tokio::select! {
            edge = edges.recv() => match edge {
                Ok(edge) => {
                    let message = edge_message(next_seq(), &edge);
                    if send(&mut sink, &message).await.is_err() { break; }
                }
                // Lagged = WE dropped events for this slow client; the seq
                // gap tells the page, and the next heartbeat resyncs it.
                Err(tokio::sync::broadcast::error::RecvError::Lagged(_)) => continue,
                Err(tokio::sync::broadcast::error::RecvError::Closed) => break,
            },
            _ = heartbeat.tick() => {
                let snapshot = snapshots.borrow().clone();
                let message = ServerMessage::Heartbeat {
                    seq: next_seq(),
                    t: shared.clock.t_us(),
                    clients: shared.clients.load(Ordering::SeqCst),
                    state: snapshot.to_protocol_state(),
                };
                if send(&mut sink, &message).await.is_err() { break; }
            },
            incoming = source.next() => match incoming {
                // Post-auth client messages: ignore unknown types (protocol
                // rule), react only to close/errors.
                Some(Ok(Message::Close(_))) | Some(Err(_)) | None => break,
                _ => continue,
            },
        }
    }

    finish(&shared);
}

fn finish(shared: &Arc<Shared>) {
    let count = shared.clients.fetch_sub(1, Ordering::SeqCst) - 1;
    println!("  client disconnected -- Clients connected: {count}");
}

fn edge_message(seq: u32, edge: &Edge) -> ServerMessage {
    match edge.kind {
        EdgeKind::ButtonDown => ServerMessage::ButtonDown {
            seq,
            t: edge.t_us,
            device: edge.device,
            index: edge.index,
        },
        EdgeKind::ButtonUp => ServerMessage::ButtonUp {
            seq,
            t: edge.t_us,
            device: edge.device,
            index: edge.index,
        },
        EdgeKind::ButtonValue(value) => ServerMessage::ButtonValue {
            seq,
            t: edge.t_us,
            device: edge.device,
            index: edge.index,
            value,
        },
        EdgeKind::Axis(value) => ServerMessage::Axis {
            seq,
            t: edge.t_us,
            device: edge.device,
            index: edge.index,
            value,
        },
    }
}

async fn send<S>(sink: &mut S, message: &ServerMessage) -> Result<(), ()>
where
    S: SinkExt<Message> + Unpin,
{
    let text = serde_json::to_string(message).map_err(|_| ())?;
    sink.send(Message::Text(text)).await.map_err(|_| ())
}

async fn close<S>(sink: &mut S, code: u16) -> Result<(), ()>
where
    S: SinkExt<Message> + Unpin,
{
    use tokio_tungstenite::tungstenite::protocol::frame::coding::CloseCode;
    use tokio_tungstenite::tungstenite::protocol::CloseFrame;
    sink.send(Message::Close(Some(CloseFrame {
        code: CloseCode::from(code),
        reason: "".into(),
    })))
    .await
    .map_err(|_| ())
}
