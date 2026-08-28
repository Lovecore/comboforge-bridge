//! The ComboForge Bridge wire protocol: types only.
//!
//! This crate is deliberately boring. It contains no I/O, no networking, no
//! platform calls, and no `unsafe` (enforced below) -- an auditor can read it
//! end to end in minutes and know it cannot do anything but describe
//! messages. The canonical human-readable spec lives in `/protocol/PROTOCOL.md`
//! at the repository root; the golden fixtures in `/protocol/fixtures/` pin
//! this implementation AND the ComboForge web page to the same bytes.
#![forbid(unsafe_code)]

pub mod message;
pub mod version;

pub use message::{
    parse_client_message, ClientMessage, ClockInfo, DeviceInfo, DeviceState, HelperInfo,
    ServerMessage,
};
pub use version::{ProtocolVersion, PROTOCOL_MAJOR, PROTOCOL_MINOR};
