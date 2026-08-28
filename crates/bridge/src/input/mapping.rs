//! XInput -> W3C Standard Gamepad. Pure, unit-tested, and the reason the
//! ComboForge page needs zero remapping: these are the exact indices Chrome
//! itself exposes for an Xbox pad, so the page's per-game token->index
//! mappings apply identically to both input sources.

use super::{DeviceSnapshot, BUTTONS};

// XInput wButtons bits (Win32 XINPUT_GAMEPAD_*).
pub const DPAD_UP: u16 = 0x0001;
pub const DPAD_DOWN: u16 = 0x0002;
pub const DPAD_LEFT: u16 = 0x0004;
pub const DPAD_RIGHT: u16 = 0x0008;
pub const START: u16 = 0x0010;
pub const BACK: u16 = 0x0020;
pub const LEFT_THUMB: u16 = 0x0040;
pub const RIGHT_THUMB: u16 = 0x0080;
pub const LEFT_SHOULDER: u16 = 0x0100;
pub const RIGHT_SHOULDER: u16 = 0x0200;
/// Only reported by the undocumented XInputGetStateEx (ordinal 100); the
/// documented call leaves this bit clear, and that is fine.
pub const GUIDE: u16 = 0x0400;
pub const A: u16 = 0x1000;
pub const B: u16 = 0x2000;
pub const X: u16 = 0x4000;
pub const Y: u16 = 0x8000;

/// (xinput bit, standard gamepad index)
const BUTTON_MAP: [(u16, usize); 15] = [
    (A, 0),
    (B, 1),
    (X, 2),
    (Y, 3),
    (LEFT_SHOULDER, 4),
    (RIGHT_SHOULDER, 5),
    // 6/7 are the analog triggers, handled separately.
    (BACK, 8),
    (START, 9),
    (LEFT_THUMB, 10),
    (RIGHT_THUMB, 11),
    (DPAD_UP, 12),
    (DPAD_DOWN, 13),
    (DPAD_LEFT, 14),
    (DPAD_RIGHT, 15),
    (GUIDE, 16),
];

/// Trigger digital thresholds: the W3C 0.5 press / 0.4 release hysteresis,
/// NOT XInput's own 30/255 -- so `pressed` semantics match what
/// navigator.getGamepads() would have said for the same hardware.
pub const TRIGGER_PRESS: f64 = 0.50;
pub const TRIGGER_RELEASE: f64 = 0.40;

/// Raw XInput state, decoupled from windows-sys so this module tests anywhere.
#[derive(Debug, Clone, Copy, Default)]
pub struct RawPad {
    pub buttons: u16,
    pub left_trigger: u8,
    pub right_trigger: u8,
    pub thumb_lx: i16,
    pub thumb_ly: i16,
    pub thumb_rx: i16,
    pub thumb_ry: i16,
}

fn stick(value: i16) -> f64 {
    (value as f64 / 32767.0).clamp(-1.0, 1.0)
}

pub fn to_snapshot(raw: RawPad) -> DeviceSnapshot {
    let mut buttons = [false; BUTTONS];
    for (bit, index) in BUTTON_MAP {
        buttons[index] = raw.buttons & bit != 0;
    }
    let triggers = [
        raw.left_trigger as f64 / 255.0,
        raw.right_trigger as f64 / 255.0,
    ];
    DeviceSnapshot {
        connected: true,
        buttons,
        triggers,
        // XInput Y is up-positive; the Standard Gamepad is up-NEGATIVE.
        axes: [
            stick(raw.thumb_lx),
            -stick(raw.thumb_ly),
            stick(raw.thumb_rx),
            -stick(raw.thumb_ry),
        ],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn dpad_lands_on_12_to_15_and_face_on_0_to_3() {
        let snap = to_snapshot(RawPad {
            buttons: DPAD_UP | DPAD_LEFT | A | Y,
            ..Default::default()
        });
        assert!(snap.buttons[12] && snap.buttons[14] && snap.buttons[0] && snap.buttons[3]);
        assert!(!snap.buttons[13] && !snap.buttons[15] && !snap.buttons[1]);
    }

    #[test]
    fn stick_up_is_negative_y() {
        let snap = to_snapshot(RawPad {
            thumb_ly: 32767,
            ..Default::default()
        });
        assert!(snap.axes[1] < -0.99);
    }

    #[test]
    fn triggers_normalize_0_to_1() {
        let snap = to_snapshot(RawPad {
            left_trigger: 255,
            right_trigger: 51,
            ..Default::default()
        });
        assert!((snap.triggers[0] - 1.0).abs() < 1e-9);
        assert!((snap.triggers[1] - 0.2).abs() < 1e-9);
    }
}
