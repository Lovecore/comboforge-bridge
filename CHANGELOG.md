# Changelog

Each release notes which protocol version it speaks.

## v0.1.0 — 2026-08-28 — protocol v1.0

- XInput poller (Windows), Standard-Gamepad normalization, trigger hysteresis,
  1/512 axis quantisation, per-connection sequence numbers.
- Loopback WebSocket server: 127.0.0.1:47821-3, origin allowlist at upgrade,
  pairing token, 1 Hz heartbeat snapshots, drop-oldest backpressure.
- `--mock` scripted pad for tests and non-Windows development.
- Golden protocol fixtures shared with the ComboForge web app.
