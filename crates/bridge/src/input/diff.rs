//! Snapshot -> edges. Pure and hammered by tests, because this is where a
//! subtle bug becomes a wrong combo on someone's stream.
//!
//! Policy, per PROTOCOL.md: buttons emit down/up on change; triggers (6/7)
//! emit buttonValue on analog change PLUS hysteresis-driven down/up; stick
//! axes are quantised to 1/512 and emitted on quantised change only. No
//! deadzone, no smoothing, ever -- the bridge is a wire, not a filter; SOCD
//! and motion logic live in the page where they are already tested.

use super::mapping::{TRIGGER_PRESS, TRIGGER_RELEASE};
use super::{DeviceSnapshot, Edge, EdgeKind, PadSnapshot, AXES, BUTTONS, MAX_DEVICES};

pub const AXIS_QUANT: f64 = 1.0 / 512.0;

pub fn quantize(value: f64) -> f64 {
    (value / AXIS_QUANT).round() * AXIS_QUANT
}

pub struct Differ {
    prev: PadSnapshot,
    trigger_pressed: [[bool; 2]; MAX_DEVICES],
}

impl Differ {
    pub fn new() -> Self {
        Differ {
            prev: PadSnapshot::empty(),
            trigger_pressed: [[false; 2]; MAX_DEVICES],
        }
    }

    pub fn step(&mut self, next: &PadSnapshot) -> Vec<Edge> {
        let mut edges = Vec::new();
        for device in 0..MAX_DEVICES {
            let prev = &self.prev.devices[device];
            let cur = &next.devices[device];
            // A device dropping out releases everything it held; a device
            // arriving announces nothing until something is actually pressed
            // (the next heartbeat snapshot carries its state anyway).
            let effective_prev = if prev.connected {
                prev.clone()
            } else {
                DeviceSnapshot::disconnected()
            };
            if !cur.connected {
                if prev.connected {
                    self.release_all(&effective_prev, device, next.t_us, &mut edges);
                }
                continue;
            }

            for index in 0..BUTTONS {
                if index == 6 || index == 7 {
                    continue; // triggers below
                }
                match (effective_prev.buttons[index], cur.buttons[index]) {
                    (false, true) => {
                        edges.push(edge(next.t_us, device, index, EdgeKind::ButtonDown))
                    }
                    (true, false) => edges.push(edge(next.t_us, device, index, EdgeKind::ButtonUp)),
                    _ => {}
                }
            }

            for trigger in 0..2 {
                let index = 6 + trigger;
                let prev_value = quantize(effective_prev.triggers[trigger]);
                let value = quantize(cur.triggers[trigger]);
                if value != prev_value {
                    edges.push(edge(next.t_us, device, index, EdgeKind::ButtonValue(value)));
                }
                let latched = &mut self.trigger_pressed[device][trigger];
                if !*latched && cur.triggers[trigger] >= TRIGGER_PRESS {
                    *latched = true;
                    edges.push(edge(next.t_us, device, index, EdgeKind::ButtonDown));
                } else if *latched && cur.triggers[trigger] < TRIGGER_RELEASE {
                    *latched = false;
                    edges.push(edge(next.t_us, device, index, EdgeKind::ButtonUp));
                }
            }

            for axis in 0..AXES {
                let prev_value = quantize(effective_prev.axes[axis]);
                let value = quantize(cur.axes[axis]);
                if value != prev_value {
                    edges.push(edge(next.t_us, device, axis, EdgeKind::Axis(value)));
                }
            }
        }
        self.prev = next.clone();
        edges
    }

    fn release_all(
        &mut self,
        prev: &DeviceSnapshot,
        device: usize,
        t_us: u64,
        edges: &mut Vec<Edge>,
    ) {
        for index in 0..BUTTONS {
            if prev.buttons[index] {
                edges.push(edge(t_us, device, index, EdgeKind::ButtonUp));
            }
        }
        for trigger in 0..2 {
            if self.trigger_pressed[device][trigger] {
                self.trigger_pressed[device][trigger] = false;
                edges.push(edge(t_us, device, 6 + trigger, EdgeKind::ButtonUp));
            }
        }
    }
}

fn edge(t_us: u64, device: usize, index: usize, kind: EdgeKind) -> Edge {
    Edge {
        t_us,
        device: device as u32,
        index: index as u32,
        kind,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn snap(t_us: u64, build: impl FnOnce(&mut DeviceSnapshot)) -> PadSnapshot {
        let mut pad = PadSnapshot::empty();
        pad.t_us = t_us;
        let mut device = DeviceSnapshot::disconnected();
        device.connected = true;
        build(&mut device);
        pad.devices[0] = device;
        pad
    }

    #[test]
    fn button_edges_carry_the_poll_timestamp() {
        let mut differ = Differ::new();
        assert!(differ.step(&snap(100, |_| {})).is_empty());
        let edges = differ.step(&snap(2100, |d| d.buttons[0] = true));
        assert_eq!(
            edges,
            vec![Edge {
                t_us: 2100,
                device: 0,
                index: 0,
                kind: EdgeKind::ButtonDown
            }]
        );
        let edges = differ.step(&snap(4100, |_| {}));
        assert_eq!(edges[0].kind, EdgeKind::ButtonUp);
    }

    #[test]
    fn trigger_hysteresis_does_not_chatter_at_the_threshold() {
        let mut differ = Differ::new();
        differ.step(&snap(0, |_| {}));
        let down = differ.step(&snap(1, |d| d.triggers[0] = 0.55));
        assert!(down
            .iter()
            .any(|e| e.kind == EdgeKind::ButtonDown && e.index == 6));
        // Resting between release (0.40) and press (0.50): no digital edge.
        let mid = differ.step(&snap(2, |d| d.triggers[0] = 0.45));
        assert!(mid
            .iter()
            .all(|e| !matches!(e.kind, EdgeKind::ButtonDown | EdgeKind::ButtonUp)));
        let up = differ.step(&snap(3, |d| d.triggers[0] = 0.30));
        assert!(up
            .iter()
            .any(|e| e.kind == EdgeKind::ButtonUp && e.index == 6));
    }

    #[test]
    fn axes_quantise_and_only_changes_emit() {
        let mut differ = Differ::new();
        differ.step(&snap(0, |_| {}));
        let first = differ.step(&snap(1, |d| d.axes[0] = 0.5001));
        assert_eq!(first.len(), 1);
        // A sub-quantum wiggle is silence.
        let wiggle = differ.step(&snap(2, |d| d.axes[0] = 0.5008));
        assert!(wiggle.is_empty());
    }

    #[test]
    fn unplugging_releases_everything_held() {
        let mut differ = Differ::new();
        differ.step(&snap(0, |d| {
            d.buttons[0] = true;
            d.triggers[1] = 1.0;
        }));
        let mut gone = PadSnapshot::empty();
        gone.t_us = 10;
        let edges = differ.step(&gone);
        assert!(edges
            .iter()
            .any(|e| e.index == 0 && e.kind == EdgeKind::ButtonUp));
        assert!(edges
            .iter()
            .any(|e| e.index == 7 && e.kind == EdgeKind::ButtonUp));
    }
}
