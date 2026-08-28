use serde::{Deserialize, Serialize};

/// Bumped only on breaking changes; the server refuses mismatched majors
/// with close code 4426 so the page can say "update the bridge" vs
/// "refresh the page" instead of failing mysteriously.
pub const PROTOCOL_MAJOR: u32 = 1;
/// Minor bumps are additive only: new optional fields, new message types.
/// Both sides MUST ignore unknown fields and unknown types (fixture-enforced).
pub const PROTOCOL_MINOR: u32 = 0;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProtocolVersion {
    pub major: u32,
    pub minor: u32,
}

impl ProtocolVersion {
    pub const CURRENT: ProtocolVersion = ProtocolVersion {
        major: PROTOCOL_MAJOR,
        minor: PROTOCOL_MINOR,
    };
}

/// The version range a client declares in its auth message.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ClientVersionRange {
    pub major: u32,
    pub minor_min: u32,
    pub minor_max: u32,
}
