# Changelog

Each release notes which protocol version it speaks.

## v0.1.2 — 2026-08-28 — protocol v1.0

- The banner always states the extra-origins list, including "none", and
  prints each entry as normalized -- an edit that did not load is visible at
  a glance instead of deducible from an absence.
- The exe reports its real version (v0.1.1's binary said 0.1.0).

## v0.1.1 — 2026-08-28 — protocol v1.0

- Origin matching normalizes hand-typed config entries (trailing slash,
  case, whitespace) against the browser's exact Origin header.
- A config file with a JSON error now fails LOUDLY and is left untouched,
  instead of being silently regenerated (which wiped edits and rotated the
  pairing token).
- The origin-rejection log line names the extraOrigins remedy for
  ComboForge deploys.

## v0.1.0 — 2026-08-28 — protocol v1.0

- XInput poller (Windows), Standard-Gamepad normalization, trigger hysteresis,
  1/512 axis quantisation, per-connection sequence numbers.
- Loopback WebSocket server: 127.0.0.1:47821-3, origin allowlist at upgrade,
  pairing token, 1 Hz heartbeat snapshots, drop-oldest backpressure.
- `--mock` scripted pad for tests and non-Windows development.
- Golden protocol fixtures shared with the ComboForge web app.
