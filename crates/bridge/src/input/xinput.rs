//! The ONLY `unsafe` in this repository: three Win32 XInput calls.
//!
//! `grep -rn "unsafe" crates/` finds this file and nothing else -- CI
//! asserts exactly that ("trust invariants" job), and every other module is
//! under `#![forbid(unsafe_code)]` or `#![deny(unsafe_code)]`.
//!
//! Why XInput and not Windows.Gaming.Input: WGI only reports to the FOCUSED
//! process, which is the exact browser limitation this program exists to
//! escape (Chrome 117+ uses WGI; the fix flag was removed in Chrome 148).
//! XInput answers any process, no window required -- it is why Chrome <=116
//! worked, and why this bridge does.
#![allow(unsafe_code)]

use std::sync::Arc;
use std::time::{Duration, Instant};

use windows_sys::Win32::UI::Input::XboxController::{XInputGetState, XINPUT_STATE};

use super::mapping::RawPad;
use super::{DeviceSnapshot, Edge, PadSnapshot, MAX_DEVICES};
use crate::clock::Clock;

const ERROR_SUCCESS: u32 = 0;
/// XInputGetState on an EMPTY slot is documented-slow (device enumeration
/// under the hood); polling four empty slots at 500 Hz would eat a core.
/// Connected slots poll at full rate; empty slots are rescanned at 1 Hz.
const DISCONNECTED_RESCAN: Duration = Duration::from_secs(1);

pub fn run(
    poll_hz: u32,
    clock: Arc<Clock>,
    edges: tokio::sync::broadcast::Sender<Edge>,
    snapshots: tokio::sync::watch::Sender<PadSnapshot>,
) {
    let period = Duration::from_secs_f64(1.0 / poll_hz as f64);
    let mut differ = super::diff::Differ::new();
    let mut connected = [false; MAX_DEVICES];
    let mut last_packet = [0u32; MAX_DEVICES];
    let mut last_rescan = Instant::now() - DISCONNECTED_RESCAN;
    let mut prev = PadSnapshot::empty();

    loop {
        let t_us = clock.t_us();
        let rescan = last_rescan.elapsed() >= DISCONNECTED_RESCAN;
        if rescan {
            last_rescan = Instant::now();
        }

        let mut pad = PadSnapshot::empty();
        pad.t_us = t_us;
        // Indexing four parallel arrays by slot; an iterator would obscure it.
        #[allow(clippy::needless_range_loop)]
        for slot in 0..MAX_DEVICES {
            if !connected[slot] && !rescan {
                continue; // keep the previous (disconnected) state
            }
            let mut state = XINPUT_STATE {
                dwPacketNumber: 0,
                Gamepad: unsafe { std::mem::zeroed() },
            };
            // SAFETY: XInputGetState writes into the struct we own and
            // returns a status code; no pointers outlive this call.
            let status = unsafe { XInputGetState(slot as u32, &mut state) };
            if status == ERROR_SUCCESS {
                connected[slot] = true;
                // dwPacketNumber is a free change detector: an unchanged
                // packet means an unchanged pad, so reuse the previous
                // snapshot and let the differ see "no change" for free.
                if state.dwPacketNumber == last_packet[slot] {
                    pad.devices[slot] = prev.devices[slot].clone();
                    continue;
                }
                last_packet[slot] = state.dwPacketNumber;
                let g = &state.Gamepad;
                pad.devices[slot] = super::mapping::to_snapshot(RawPad {
                    buttons: g.wButtons,
                    left_trigger: g.bLeftTrigger,
                    right_trigger: g.bRightTrigger,
                    thumb_lx: g.sThumbLX,
                    thumb_ly: g.sThumbLY,
                    thumb_rx: g.sThumbRX,
                    thumb_ry: g.sThumbRY,
                });
            } else {
                connected[slot] = false;
                pad.devices[slot] = DeviceSnapshot::disconnected();
            }
        }
        // Slots we skipped this tick keep their previous state.
        #[allow(clippy::needless_range_loop)]
        for slot in 0..MAX_DEVICES {
            if !connected[slot] && !rescan {
                pad.devices[slot] = prev.devices[slot].clone();
            }
        }

        for edge in differ.step(&pad) {
            let _ = edges.send(edge);
        }
        let _ = snapshots.send(pad.clone());
        prev = pad;

        // v0 cadence: plain sleep. v1 upgrades to a high-resolution waitable
        // timer (CreateWaitableTimerExW + CREATE_WAITABLE_TIMER_HIGH_RESOLUTION)
        // for sub-ms periods without the globally rude timeBeginPeriod(1).
        std::thread::sleep(period);
    }
}
