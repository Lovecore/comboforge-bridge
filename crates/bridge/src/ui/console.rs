//! v1 is console-only ON PURPOSE: the console window is the transparency UI.
//! A tray icon hides what a program is doing behind a tooltip; this window
//! shows you, live and in plain text, every socket, every client, and every
//! rejected origin. Closing the window stops the bridge -- the most obvious
//! off switch there is. (Tray arrives in v1.1, opt-in.)

use crate::config::{config_path, Config};

pub fn banner(config: &Config, poll_hz: u32, mock: bool) {
    println!();
    println!(
        "ComboForge Bridge {}   (github.com/Lovecore/comboforge-bridge)",
        env!("CARGO_PKG_VERSION")
    );
    println!();
    if mock {
        println!("  MODE        --mock: a scripted pad, no real controller is read");
    }
    println!("  Poll rate   {poll_hz} Hz");
    println!(
        "  Pairing     {}          <- paste this into the ComboForge page",
        config.token
    );
    println!("  Config      {}", config_path().display());
    // Always printed, so "my edit did not load" is visible at a glance
    // rather than deducible from an absence.
    if config.extra_origins.is_empty() {
        println!("  Extra origins  none (production origins only)");
    } else {
        println!("  EXTRA ORIGINS from config (review these!):");
        for origin in &config.extra_origins {
            println!(
                "    + {}",
                crate::server::handshake::normalize_origin(origin)
            );
        }
    }
    println!();
    println!("  This program never connects to the internet. Verify it yourself:");
    println!("  docs/VERIFY.md in the repository.");
    println!();
    println!("  Close this window to stop the bridge.");
    println!();
}
