//! Every socket this program ever opens is in this module: ONE
//! `TcpListener::bind`, on 127.0.0.1, and no connect() anywhere in the tree
//! (CI greps for it). Chrome 147+ may ask the user's permission before a
//! page can reach us (Local Network Access) -- a Block over there looks like
//! "not running" over here, which is why the console prints the connected
//! client count: it answers that question from this side.

pub mod client;
pub mod handshake;

use std::net::{Ipv4Addr, SocketAddr, SocketAddrV4};
use std::sync::atomic::{AtomicU32, Ordering};
use std::sync::Arc;

use tokio::net::TcpListener;

use crate::clock::Clock;
use crate::config::Config;
use crate::input::{Edge, PadSnapshot};

pub const PORTS: [u16; 3] = [47821, 47822, 47823];
pub const MAX_CLIENTS: u32 = 4;

pub struct Shared {
    pub config: Config,
    pub clock: Arc<Clock>,
    pub poll_hz: u32,
    pub clients: AtomicU32,
}

pub async fn run(
    config: Config,
    clock: Arc<Clock>,
    poll_hz: u32,
    edges: tokio::sync::broadcast::Sender<Edge>,
    snapshots: tokio::sync::watch::Receiver<PadSnapshot>,
) {
    let mut listener = None;
    for port in PORTS {
        // 127.0.0.1 LITERALLY: not "localhost" (may resolve to ::1), not
        // 0.0.0.0. One socket, one address family, loopback only --
        // verifiable from PowerShell, see docs/VERIFY.md.
        let addr = SocketAddr::V4(SocketAddrV4::new(Ipv4Addr::LOCALHOST, port));
        match TcpListener::bind(addr).await {
            Ok(l) => {
                println!("  Listening   ws://127.0.0.1:{port}   (loopback only -- no other socket is open)");
                listener = Some(l);
                break;
            }
            Err(e) => eprintln!("  port {port} unavailable ({e}); trying next"),
        }
    }
    let Some(listener) = listener else {
        eprintln!("no port available out of {PORTS:?}; is another bridge already running?");
        std::process::exit(1);
    };

    let shared = Arc::new(Shared {
        config,
        clock,
        poll_hz,
        clients: AtomicU32::new(0),
    });

    loop {
        let Ok((stream, _)) = listener.accept().await else {
            continue;
        };
        let shared = Arc::clone(&shared);
        let edges = edges.subscribe();
        let snapshots = snapshots.clone();
        tokio::spawn(async move {
            if shared.clients.load(Ordering::SeqCst) >= MAX_CLIENTS {
                // Politeness cap; a fifth client is almost certainly a bug.
                return;
            }
            client::serve(stream, shared, edges, snapshots).await;
        });
    }
}
