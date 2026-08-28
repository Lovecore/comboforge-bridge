# The dependency budget

DEPENDENCY_CAP=65

Current tree size: 62 crates (`cargo tree --edges normal --prefix none | sort -u | wc -l`).

CI fails if the tree exceeds the cap above. Growing it requires a commit that
visibly raises this number — which is the point: every new transitive crate is
something an auditor has to trust, and this file makes adding one a decision
instead of an accident.

Notable absences, by design:

- **No TLS stack** (tokio-tungstenite `default-features = false`) — the binary
  is structurally incapable of https.
- **No HTTP client** of any kind.
- **No gamepad abstraction crate** — XInput is three FFI calls, and a mapping
  database we don't need is audit surface we don't want.

Regenerate the snapshot with:

```
cargo tree --edges normal
```
