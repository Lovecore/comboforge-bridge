use serde::{Deserialize, Serialize};

use crate::version::{ClientVersionRange, ProtocolVersion};

/// One connected controller, as announced in `hello`.
///
/// `mapping` is always `"standard"` in v1: the bridge normalizes XInput to
/// the W3C Standard Gamepad layout (the same one Chrome exposes for Xbox
/// pads), so the page's per-game token->index mappings apply unchanged and
/// its hat-switch decoding knows to stay out of the way.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceInfo {
    pub device: u32,
    pub id: String,
    pub mapping: String,
    pub buttons: u32,
    pub axes: u32,
    pub connected: bool,
}

/// A full state snapshot for one device. Sent in `hello` and every
/// heartbeat: it is the resync point after any dropped event.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct DeviceState {
    pub device: u32,
    pub buttons: Vec<bool>,
    pub axes: Vec<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct HelperInfo {
    pub name: String,
    pub version: String,
}

/// The helper's clocks, for the page's offset estimation (min-filter over
/// hello + heartbeats, NTP-style; see PROTOCOL.md "Clock alignment").
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClockInfo {
    /// Microseconds since helper start, monotonic.
    pub t_us: u64,
    /// Wall clock at the same instant, milliseconds since the Unix epoch.
    pub wall_ms: u64,
    pub poll_hz: u32,
}

/// Everything the server ever sends. Tagged by `type`.
///
/// Field order in these structs IS the wire order (serde serializes in
/// declaration order), and the golden fixtures assert the bytes -- reorder a
/// field and CI fails on both repos.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ServerMessage {
    #[serde(rename = "hello")]
    Hello {
        protocol: ProtocolVersion,
        helper: HelperInfo,
        clock: ClockInfo,
        devices: Vec<DeviceInfo>,
        state: Vec<DeviceState>,
    },
    /// A digital press. `t` is microseconds since helper start, sampled at
    /// the POLL instant -- at 500 Hz that is 2ms quantisation vs the
    /// browser's ~16.7ms rAF grid, which is the whole reason to run a bridge.
    #[serde(rename = "buttonDown")]
    ButtonDown {
        seq: u32,
        t: u64,
        device: u32,
        index: u32,
    },
    #[serde(rename = "buttonUp")]
    ButtonUp {
        seq: u32,
        t: u64,
        device: u32,
        index: u32,
    },
    /// Analog trigger travel (indices 6/7), emitted alongside the
    /// hysteresis-driven down/up (press >= 0.50, release < 0.40 -- the W3C
    /// convention, so `pressed` matches navigator.getGamepads()).
    #[serde(rename = "buttonValue")]
    ButtonValue {
        seq: u32,
        t: u64,
        device: u32,
        index: u32,
        value: f64,
    },
    /// Stick axis, quantised to 1/512 steps, emitted on change only.
    /// Never deadzoned, never smoothed: the bridge is a wire, not a filter.
    #[serde(rename = "axis")]
    Axis {
        seq: u32,
        t: u64,
        device: u32,
        index: u32,
        value: f64,
    },
    /// 1 Hz always, even idle: liveness, seq-gap resync, and the connected
    /// client count the helper also displays (the two ends of the same
    /// trust indicator).
    #[serde(rename = "heartbeat")]
    Heartbeat {
        seq: u32,
        t: u64,
        clients: u32,
        state: Vec<DeviceState>,
    },
    #[serde(rename = "error")]
    Error { code: String, message: String },
}

/// Everything a client may send. Unknown types are ignored, never fatal.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type")]
pub enum ClientMessage {
    /// Must arrive within 3000ms of the upgrade or the server closes 4408.
    /// The token travels in this first message rather than a
    /// challenge-response: there is no eavesdropper on loopback, and this
    /// repository trades cryptographic theatre for readability on purpose
    /// (PROTOCOL.md says so in more words).
    #[serde(rename = "auth")]
    Auth {
        token: String,
        protocol: ClientVersionRange,
        client: String,
    },
}

/// Tolerant client-message parsing: a malformed or unknown message is
/// `None`, and the caller decides whether that is ignorable (unknown type)
/// or suspicious (pre-auth garbage).
pub fn parse_client_message(raw: &str) -> Option<ClientMessage> {
    serde_json::from_str(raw).ok()
}
