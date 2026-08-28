//! A scripted pad behind `--mock`: CI runners have no controllers, and
//! neither does the macOS machine this repo is developed on. The script
//! loops a recognisable sequence (neutral, a press, a quarter-circle into a
//! press) so an end-to-end test -- or a human watching the ComboForge page
//! -- can see something combo-shaped without hardware.

use std::sync::Arc;

use super::{DeviceSnapshot, Edge, PadSnapshot};
use crate::clock::Clock;

/// (millis into the loop, buttons pressed, dpad [up,down,left,right])
const SCRIPT: &[(u64, &[usize], [bool; 4])] = &[
    (0, &[], [false, false, false, false]),
    (400, &[0], [false, false, false, false]), // A
    (520, &[], [false, false, false, false]),
    (900, &[], [false, true, false, false]),   // 2
    (960, &[], [false, true, false, true]),    // 3
    (1020, &[], [false, false, false, true]),  // 6
    (1080, &[2], [false, false, false, true]), // X with 6 held
    (1200, &[], [false, false, false, false]),
    (2000, &[], [false, false, false, false]),
];
const LOOP_MS: u64 = 2400;

fn state_at(ms: u64) -> DeviceSnapshot {
    let ms = ms % LOOP_MS;
    let mut current = SCRIPT[0];
    for entry in SCRIPT {
        if entry.0 <= ms {
            current = *entry;
        }
    }
    let mut device = DeviceSnapshot::disconnected();
    device.connected = true;
    for &button in current.1 {
        device.buttons[button] = true;
    }
    let [up, down, left, right] = current.2;
    device.buttons[12] = up;
    device.buttons[13] = down;
    device.buttons[14] = left;
    device.buttons[15] = right;
    device
}

pub fn run(
    poll_hz: u32,
    clock: Arc<Clock>,
    edges: tokio::sync::broadcast::Sender<Edge>,
    snapshots: tokio::sync::watch::Sender<PadSnapshot>,
) {
    let period = std::time::Duration::from_secs_f64(1.0 / poll_hz as f64);
    let mut differ = super::diff::Differ::new();
    loop {
        let t_us = clock.t_us();
        let mut pad = PadSnapshot::empty();
        pad.t_us = t_us;
        pad.devices[0] = state_at(t_us / 1000);
        for edge in differ.step(&pad) {
            // A send error just means no client is listening; keep polling.
            let _ = edges.send(edge);
        }
        let _ = snapshots.send(pad);
        std::thread::sleep(period);
    }
}
