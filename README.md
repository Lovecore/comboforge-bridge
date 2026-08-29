# ComboForge Bridge

Reads your controller and streams the presses to [ComboForge](https://comboforge.gg)
pages running on **your own machine** so combo capture and the input overlay keep
working while your game has focus.

It has to exist because of a browser limitation: since Chrome 117, Windows only
delivers controller input to the *focused* window
([crbug 392661398](https://issues.chromium.org/issues/392661398)), so a ComboForge
tab goes deaf the moment you click into your game. The switch that used to fix it
was removed in Chrome 148. This program reads the controller the way games do
(XInput, which answers any process) and hands it to the page over a local socket.

## In plain terms

This little program does exactly one thing: it listens to your game
controller and tells the ComboForge page on your screen which buttons you
are pressing. That's the whole job.

- **It only talks to ComboForge, and only on your own computer.** Nothing
  ever goes out to the internet. Ever. Not stats, not updates, not crash
  reports -- nothing.
- **It cannot see your keyboard, your mouse, your screen, your files, or
  your passwords.** It doesn't have the code to do any of that, and it never
  will.
- **Closing its window turns it off completely.** There is no background
  service and nothing left running.
- **You don't have to take our word for any of this.** The entire program is
  public code that anyone can read, and everything below shows how to check
  each claim yourself -- or ask any programmer you trust to look; it's
  short on purpose.

## What this program can see, and where it goes (the precise version)

```
It reads:   your Xbox-compatible controller, via XInput. That's it.
It sends:   controller press/release events, over a WebSocket bound to
            127.0.0.1, to web pages ON YOUR OWN MACHINE that present the
            pairing token shown in this program's window.
It writes:  exactly one file — its own config.json (the pairing token).
It never:   makes an outbound network connection of any kind.
            resolves a hostname.
            checks for updates.
            reads your keyboard or your mouse.
            reads your screen, your files, or any other process.
            writes to the registry.
```

**Keyboard and mouse support is a non-goal, permanently.** Not "not yet" just never.
A program that reads your keyboard is a keylogger with a nice README, and no
amount of open source makes that a comfortable thing to run. Controllers only.

**This program never checks for updates, and it never will.** An updater is an
outbound connection, a download, and code that replaces the binary you audited
with one you didn't. The ComboForge page tells you when a new version exists
the page is already talking to the internet, and this program shouldn't be.

These claims are **tested**: the CI "trust invariants" job fails (and auditable in this repo)
the build if outbound-connection code appears anywhere in the tree, or if
`unsafe` appears outside the one file documented below. Found a claim in this
README that is not true of the code? **That is a security report** — see
[SECURITY.md](SECURITY.md).

## Verify all of that yourself (60 seconds)

See [docs/VERIFY.md](docs/VERIFY.md) just copy-paste PowerShell that shows the one
loopback socket, invites you to firewall the program's outbound traffic
permanently (nothing changes, because it never dials), and checks that the exe
you downloaded was built by public CI from this exact source:

```
gh attestation verify comboforge-bridge.exe --repo Lovecore/comboforge-bridge
```

## Reading the source: the map

The whole program is small on purpose. If you only read four files:

- [`crates/bridge/src/input/xinput.rs`](crates/bridge/src/input/xinput.rs) —
  **the only `unsafe` code in this repository**: the Win32 XInput read.
  `grep -rlE 'unsafe (fn|impl|trait)|unsafe \{' crates/` finds this file and
  nothing else (every other module carries `forbid(unsafe_code)` or
  `deny(unsafe_code)`); CI asserts it on every push.
- [`crates/bridge/src/server/mod.rs`](crates/bridge/src/server/mod.rs) — every
  socket this program ever opens. One `TcpListener::bind`, on `127.0.0.1`, and
  no `connect` anywhere in the tree.
- [`crates/bridge/src/server/handshake.rs`](crates/bridge/src/server/handshake.rs)
  — the origin allowlist and the pairing-token check, as pure tested functions.
- [`crates/bridge/src/config.rs`](crates/bridge/src/config.rs) — every
  filesystem operation. One path, one file.

The wire protocol is specified in [`protocol/PROTOCOL.md`](protocol/PROTOCOL.md);
[`crates/protocol`](crates/protocol) contains the message types only (no I/O, no
`unsafe`, `#![forbid(unsafe_code)]`) and both this repo and the ComboForge web
app test against the same golden fixtures.

## Install

1. Download `comboforge-bridge.exe` from
   [Releases](https://github.com/Lovecore/comboforge-bridge/releases) —
   ideally verify it first (one command, above).
2. Run it. A console window opens showing your pairing token.
   **Windows will warn you the first time** ("Windows protected your PC" →
   More info → Run anyway). That means "Microsoft hasn't seen this file
   downloaded many times yet," not "this file is dangerous" — every new release
   starts at zero reputation. If that makes you uncomfortable: good instinct.
   Verify the file first.
3. On the ComboForge Stream Capture page, paste the pairing token when asked.
   Chrome may show a "local network" permission prompt, this is the browser
   asking whether this page may talk to programs on your machine; allow it.
4. Play. The console window shows connected clients live; close it to stop.

To start it with Windows, drop a shortcut in `shell:startup` (Win+R, type
`shell:startup`). The program deliberately does not offer to do this itself in
v1 — it writes no registry keys at all.

## Troubleshooting

- **"Your config file has a JSON error"** -- the window names the file, line
  and column. The three typos that account for nearly all of these: a URL in
  `extraOrigins` missing its quotation marks (every value needs `"quotes"`),
  a trailing comma after the last entry, or a missing comma between the
  token line and `extraOrigins`. A working example:

  ```json
  {
    "token": "AAAA-BBBB-CCCC-DDDD-EEEE",
    "extraOrigins": ["https://your-preview.vercel.app"]
  }
  ```

- **"ORIGIN REJECTED" in the log** -- the page connecting is not on the
  allowlist. Production ComboForge always is; a preview/staging deploy needs
  its origin added to `extraOrigins` (restart afterwards -- the banner lists
  what loaded).
- **The ComboForge page says the Bridge is not running, but it is** -- your
  browser may have blocked the page's "local network" permission. Click the
  icon left of the address bar and allow it. The Bridge's own client count
  is the tiebreaker: if it says 0 while the page tries to connect, the
  browser is blocking; if it counts up, the page is connected.

## Building from source

```
cargo build --release
```

`rust-toolchain.toml` pins the toolchain; `Cargo.lock` is committed. See
[docs/BUILD.md](docs/BUILD.md) — why we ship
provenance attestation instead of claiming bit-for-bit reproducibility.

On macOS/Linux the binary builds and runs with `--mock` (a scripted pad, used
by the tests) but cannot read real controllers — the browser limitation this
exists to fix is **Windows-specific**, and a cross-platform build would be surface
with no beneficiary.

## Threat model

[docs/THREAT-MODEL.md](docs/THREAT-MODEL.md) — including the entries:
a same-user process gains nothing from this program it couldn't get by calling
XInput itself, and a compromised ComboForge page would legitimately hold your
pairing token locally. Read it before trusting us; that's what it's for.

## License

MIT OR Apache-2.0, at your option. No CLA — a CLA on a "trust me" project
signals relicensing risk, and this project's whole premise is that you should
not have to trust anyone.
