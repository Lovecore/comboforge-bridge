# Threat model

Structure: attacker → what they try → what stops them → what remains. The
"what remains" column is the point — a threat model that ends every row with
"nothing" is marketing.

| Attacker | Attempt | Mitigation | Residual risk |
|---|---|---|---|
| Any web page you visit | `new WebSocket("ws://127.0.0.1:47821")` to read your inputs | The browser sets the `Origin` header and page JavaScript cannot forge it. Non-allowlisted origins get HTTP 403 and the connection never upgrades. Chrome 147+ additionally gates the attempt behind a user permission prompt. | A compromised (XSS'd) comboforge.gg passes the origin check, and the page legitimately holds your pairing token — so XSS on ComboForge reads your controller inputs. Controller inputs. |
| Another native process running as you | Forge `Origin`, connect, read inputs | The pairing token (100 bits, OS RNG) | **It can read config.json — same user, same permissions.** But it could also simply call `XInputGetState` itself, no bridge needed. **This program grants a local attacker nothing they did not already have.** |
| Another user on the same machine | Connect from their session | Loopback is per-machine, so the port is reachable — but the token lives under your `%LOCALAPPDATA%`, ACL'd to you | A local administrator can read anything and is out of scope: an admin can install an actual keylogger. |
| Someone on your LAN | Connect from another machine | `bind(127.0.0.1)` — not reachable off-box, verifiable with `Get-NetTCPConnection` | None, unless you install a port forwarder yourself. |
| A malicious release | Put a backdoored exe on the releases page | Build-provenance attestation: releases are built by public CI from public source, and `gh attestation verify` checks the file hash against a public transparency log | You have to actually run the verify command. It is one line and it sits above the download link. |
| The supply chain | A malicious transitive crate | Committed `Cargo.lock`, `--locked` builds, `cargo-deny` in CI, a dependency-count budget that fails CI when the tree grows, no TLS stack at all | Real and not eliminable. Mitigated by keeping the tree small enough that a human can actually read `cargo tree` — that is what the budget gate protects. |

## Explicitly out of scope (with reasons, not dismissals)

- **An attacker already running code as your user.** They have XInput, your
  files, and your browser. The bridge adds nothing to their capabilities.
- **A compromised browser or malicious extension.** It sits between you and
  every page; no local program can defend that boundary.
- **Physical access.**
- **A legitimately-paired ComboForge page doing what you paired it to do.**
  Pairing means "this page may read my controller." That is the product.

## Non-goals, stated as permanent commitments

No keyboard capture. No mouse capture. No screen capture. No file access
beyond one config file. No outbound network. No auto-update. No telemetry.
No crash reporting. Each of those is a thing people are right to be nervous
about, named and refused. If a future version of this program wants any of
them, treat the request itself as suspicious.
