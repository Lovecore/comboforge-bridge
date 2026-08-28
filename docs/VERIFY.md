# Prove it yourself (60 seconds)

The README makes claims. Here is how you check them on your own machine, with
nothing but PowerShell and (for step 4) the GitHub CLI.

## 1. It listens on loopback only, and connects nowhere

```powershell
Get-NetTCPConnection -OwningProcess (Get-Process comboforge-bridge).Id |
  Format-Table LocalAddress,LocalPort,RemoteAddress,State
```

Expect exactly one row: `127.0.0.1 : 47821 : 0.0.0.0 : Listen` (or 47822/47823
if 47821 was taken). Any row with `State = Established` and a remote address
that is not `127.0.0.1` would be a bug — and a security report we want.

## 2. Watch it over time

Sysinternals TCPView, or:

```powershell
while ($true) { netstat -ano | Select-String (Get-Process comboforge-bridge).Id; sleep 2 }
```

Connections appear only when a ComboForge page on this machine connects, and
their remote address is always 127.0.0.1.

## 3. Cut it off from the internet permanently, and lose nothing

```powershell
New-NetFirewallRule -DisplayName "Block ComboForge Bridge outbound" -Direction Outbound `
  -Program "C:\path\to\comboforge-bridge.exe" -Action Block
```

The program behaves identically with this rule in place, because it never
dials out. We invite you to leave the rule there forever.

## 4. The exe you downloaded came from this public source

```powershell
gh attestation verify comboforge-bridge.exe --repo Lovecore/comboforge-bridge
```

This checks a cryptographically signed statement — recorded in a public
transparency log at release time — that this exact file (by hash) was produced
by this repository's public release workflow, at a named commit, on GitHub's
own runners. If the file was modified by anyone after CI built it, the
verification fails.

## 5. The binary cannot speak TLS

Not a promise — a build property. `crates/bridge/Cargo.toml` enables
tokio-tungstenite with `default-features = false`: no `native-tls`, no
`rustls`. There is no TLS implementation in the binary to misuse:

```
cargo tree | Select-String -Pattern "rustls|native-tls|openssl"
```

returns nothing.
