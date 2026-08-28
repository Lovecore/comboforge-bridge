# Building

```
cargo build --release          # the exe lands in target/release/
cargo test                     # includes a full no-hardware end-to-end run
cargo run -- --mock            # scripted pad, works on any OS
```

`rust-toolchain.toml` pins the exact toolchain; `Cargo.lock` is committed and
CI builds `--locked`. Release binaries are built by the public release
workflow on GitHub-hosted runners and attested (see docs/VERIFY.md §4).

## On reproducible builds — honestly

**We are not bit-for-bit reproducible, and we are not going to claim we are.**
Rust binaries on Windows/MSVC are not deterministic today
(rust-lang/rust#88982) — two builds of a dependency-free hello-world differ in
the same environment. This is a known upstream limitation, not something we
have been lazy about.

We do the cheap parts anyway, because they shrink the surface: pinned
toolchain, committed lockfile, `SOURCE_DATE_EPOCH` (zeroes the PE timestamp
via `/Brepro`), `--remap-path-prefix`, stripped symbols.

Instead of a reproducibility claim we cannot back, we give you provenance:
GitHub cryptographically signs a statement that this exact file hash was
produced by this exact workflow from this exact public commit, into a public
transparency log, and one command checks it. That is a weaker guarantee than
reproducibility in theory and a stronger one in practice — because nobody was
ever actually going to rebuild it and diff.
