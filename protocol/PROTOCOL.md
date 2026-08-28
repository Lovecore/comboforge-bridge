# ComboForge Bridge protocol — v1.0

This document is canonical. The Rust types in `crates/protocol` and the
ComboForge web app's parser are both pinned to `fixtures/v1.jsonl`
(byte-exact golden tests on both repos' CI); the fixture file is also
published as a release asset with its SHA256 in the release notes.

## Transport

- `ws://127.0.0.1:47821`, falling back to `47822`, then `47823` (all below
  Windows' 49152 ephemeral floor, out of Hyper-V's reservation range).
  Clients probe the three ports in order.
- The server binds `127.0.0.1` literally — one socket, one address family.
- Text frames, one JSON message per frame. No binary frames in v1.

## Handshake

```
client connects  →  server checks the Origin header of the HTTP upgrade
                    ├─ absent or not allowlisted → HTTP 403 (never upgrades)
                    └─ allowed → upgrade completes
client sends     →  auth (below) within 3000ms, or the server closes 4408
server replies   →  hello         on success
                    error{badToken} then close 4401 on a wrong token
                    error{protocolMismatch} then close 4426 on a major mismatch
```

The compiled-in origin allowlist is `https://comboforge.gg`,
`https://www.comboforge.gg`, `http://localhost:5173`, `http://127.0.0.1:5173`;
the helper's config may ADD origins (announced loudly at startup) and can
never remove these. A missing Origin is rejected: browsers always send it and
page JavaScript cannot forge it. Native clients can forge anything — see the
threat model for why that is acceptable.

The token travels in the first message rather than a challenge-response, on
purpose: there is no eavesdropper on loopback, and this project trades
cryptographic theatre for a handshake anyone can read. Five failed auths
close the connection and impose a 5-second backoff on new ones.

### auth (client → server)

```json
{"type":"auth","token":"KX7M-9QT2-4BNH-5RWD-J3PC","protocol":{"major":1,"minorMin":0,"minorMax":0},"client":"comboforge-web/2026.08"}
```

## hello (server → client)

See `fixtures/v1.jsonl` line 1 for the exact shape. Notes:

- `clock.tUs` — microseconds since helper start (monotonic); `clock.wallMs` —
  wall clock at the same instant; `clock.pollHz` — the poll cadence.
- `devices[].mapping` is always `"standard"` in v1: indices are the W3C
  Standard Gamepad layout (the same one Chrome exposes for Xbox pads).
  Consumers must NOT run hat-switch heuristics against bridge devices.
- `state` carries a full snapshot per connected device — the initial resync
  point.

## Events (server → client)

```json
{"type":"buttonDown","seq":1,"t":41824773,"device":0,"index":2}
{"type":"buttonUp","seq":2,"t":41824990,"device":0,"index":2}
{"type":"buttonValue","seq":4,"t":41826019,"device":0,"index":7,"value":0.423828125}
{"type":"axis","seq":5,"t":41826019,"device":0,"index":0,"value":-0.984375}
```

- `seq`: monotonic u32 **per connection**, starting at 1, wrapping at u32 max.
  A gap means the server dropped events for THIS client under backpressure;
  resync from the next heartbeat's snapshot.
- `t`: microseconds since helper start, sampled **at the poll instant** — at
  500 Hz that is 2ms quantisation against the browser's ~16.7ms rAF grid.
- Digital buttons emit down/up. Triggers (indices 6/7) additionally emit
  `buttonValue` on analog change, and their down/up uses hysteresis: press at
  ≥ 0.50, release at < 0.40 (the W3C convention, so `pressed` matches
  `navigator.getGamepads()`; deliberately NOT XInput's 30/255 threshold).
- Axes are quantised to 1/512 steps and emitted on quantised change only.
  **No deadzone, no smoothing, ever** — the bridge is a wire, not a filter;
  SOCD resolution and motion recognition belong to the consumer.

## heartbeat (server → client)

1 Hz, always, even idle. Carries `clients` (the connected-client count — the
same number the helper displays, which is how a user distinguishes "browser
blocked the connection" from "helper not running") and a full `state`
snapshot per device for resync.

## Clock alignment (consumer guidance, normative for ComboForge)

Estimate `offset = min over samples of (performanceNowAtReceipt - t/1000)`
across hello and heartbeats (an NTP-style min filter: send jitter only ever
inflates the sample, so the minimum converges on the true offset). Map any
event to page time as `t/1000 + offset`. Re-anchor continuously; drift over a
session is negligible but free to correct.

## Versioning

- Same `major` = compatible. The server closes mismatched majors with 4426
  and names its major, so the consumer can say "update the bridge" versus
  "refresh the page".
- Minor bumps are additive only: new optional fields, new message types.
- **Both sides MUST ignore unknown message types and unknown fields.** The
  fixture file contains a fabricated future-type line that consumers must
  skip without error; the Rust golden test skips it by design.
- The helper's semver and the protocol version are independent; each release
  states which protocol it speaks in CHANGELOG.md.

## Close codes

| Code | Meaning |
|---|---|
| 4400 | pre-auth garbage |
| 4401 | bad token |
| 4408 | auth timeout (3s) |
| 4426 | protocol major mismatch |
