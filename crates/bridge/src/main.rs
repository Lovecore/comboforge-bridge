//! ComboForge Bridge: reads your controller, streams it to ComboForge pages
//! on YOUR machine, and does nothing else. The README's claims about this
//! program are enforced by CI (see .github/workflows/ci.yml "trust
//! invariants"); if you find one that is not true of the code, that is a
//! security report.
#![deny(unsafe_code)] // the ONE exception lives in input/xinput.rs

mod clock;
mod config;
mod input;
mod server;
mod token;
mod ui;

use std::sync::Arc;

fn main() {
    let args: Vec<String> = std::env::args().collect();
    let mock = args.iter().any(|a| a == "--mock");
    let poll_hz: u32 = arg_value(&args, "--poll-hz")
        .and_then(|v| v.parse().ok())
        .unwrap_or(500);

    let config = config::load_or_create();
    let clock = Arc::new(clock::Clock::start());

    ui::console::banner(&config, poll_hz, mock);

    // The poller is a plain thread on a fixed cadence; the server is tokio.
    // They meet at a broadcast channel of edges plus a watch of the latest
    // snapshot -- broadcast's drop-oldest-on-lag IS the backpressure policy
    // (a stalled browser tab must never add latency to anyone else).
    let (edge_tx, _) = tokio::sync::broadcast::channel::<input::Edge>(1024);
    let (snap_tx, snap_rx) = tokio::sync::watch::channel(input::PadSnapshot::empty());

    {
        let edge_tx = edge_tx.clone();
        let clock = Arc::clone(&clock);
        std::thread::Builder::new()
            .name("poller".into())
            .spawn(move || {
                if mock {
                    input::mock::run(poll_hz, clock, edge_tx, snap_tx);
                } else {
                    #[cfg(windows)]
                    input::xinput::run(poll_hz, clock, edge_tx, snap_tx);
                    #[cfg(not(windows))]
                    {
                        drop((edge_tx, snap_tx));
                        eprintln!(
                            "Real controller input requires Windows (XInput). \
                             On this platform run with --mock for a scripted pad."
                        );
                        std::process::exit(2);
                    }
                }
            })
            .expect("spawn poller");
    }

    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .expect("tokio runtime");
    runtime.block_on(server::run(config, clock, poll_hz, edge_tx, snap_rx));
}

fn arg_value<'a>(args: &'a [String], name: &str) -> Option<&'a str> {
    args.iter()
        .position(|a| a == name)
        .and_then(|i| args.get(i + 1))
        .map(|s| s.as_str())
}
