//! Input pipeline: a poller (real XInput on Windows, scripted mock anywhere)
//! produces `PadSnapshot`s on a fixed cadence; `diff` turns consecutive
//! snapshots into edge events. Everything except `xinput.rs` is pure and
//! platform-free, which is what lets CI and macOS development run the whole
//! server end-to-end with no hardware.

pub mod diff;
// On non-Windows builds the real poller is absent, so the mapping module is
// reached only by tests -- the allow keeps that platform truth from reading
// as dead code.
#[cfg_attr(not(windows), allow(dead_code))]
pub mod mapping;
pub mod mock;
#[cfg(windows)]
pub mod xinput;

pub const MAX_DEVICES: usize = 4;
pub const BUTTONS: usize = 17; // W3C Standard Gamepad incl. index 16 (Guide)
pub const AXES: usize = 4;

/// One device at one poll instant, already in Standard Gamepad layout.
#[derive(Debug, Clone, PartialEq)]
pub struct DeviceSnapshot {
    pub connected: bool,
    pub buttons: [bool; BUTTONS],
    /// Analog trigger travel for indices 6/7, 0.0..=1.0.
    pub triggers: [f64; 2],
    /// Sticks: [lx, ly, rx, ry], -1.0..=1.0, up = NEGATIVE y (W3C convention;
    /// XInput's up-positive is inverted in mapping.rs).
    pub axes: [f64; AXES],
}

impl DeviceSnapshot {
    pub fn disconnected() -> Self {
        DeviceSnapshot {
            connected: false,
            buttons: [false; BUTTONS],
            triggers: [0.0; 2],
            axes: [0.0; AXES],
        }
    }
}

/// All four slots at one poll instant.
#[derive(Debug, Clone, PartialEq)]
pub struct PadSnapshot {
    pub t_us: u64,
    pub devices: [DeviceSnapshot; MAX_DEVICES],
}

impl PadSnapshot {
    pub fn empty() -> Self {
        PadSnapshot {
            t_us: 0,
            devices: [
                DeviceSnapshot::disconnected(),
                DeviceSnapshot::disconnected(),
                DeviceSnapshot::disconnected(),
                DeviceSnapshot::disconnected(),
            ],
        }
    }

    pub fn to_protocol_state(&self) -> Vec<comboforge_bridge_protocol::DeviceState> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| d.connected)
            .map(|(i, d)| comboforge_bridge_protocol::DeviceState {
                device: i as u32,
                buttons: d.buttons.to_vec(),
                axes: d.axes.to_vec(),
            })
            .collect()
    }

    pub fn to_protocol_devices(&self) -> Vec<comboforge_bridge_protocol::DeviceInfo> {
        self.devices
            .iter()
            .enumerate()
            .filter(|(_, d)| d.connected)
            .map(|(i, _)| comboforge_bridge_protocol::DeviceInfo {
                device: i as u32,
                id: format!("XInput Slot {i}"),
                mapping: "standard".into(),
                buttons: BUTTONS as u32,
                axes: AXES as u32,
                connected: true,
            })
            .collect()
    }
}

/// One observed change, unsequenced -- `seq` is per-connection and stamped
/// by each client task, so a late joiner starts at 1 like everyone else.
#[derive(Debug, Clone, PartialEq)]
pub enum EdgeKind {
    ButtonDown,
    ButtonUp,
    ButtonValue(f64),
    Axis(f64),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Edge {
    pub t_us: u64,
    pub device: u32,
    pub index: u32,
    pub kind: EdgeKind,
}
