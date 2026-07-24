# What rx is

**rx is a fast local-CI and task runner for Rust workspaces.**

Its job:

1. Define the project's tasks once, in `rx.toml`.
2. Run the same pipeline locally and in CI (`rx ci` = `rx run ci`).
3. Determine which workspace packages are affected by a change.
4. Execute independent tasks concurrently.
5. Delegate compilation scheduling and compiler caching to Cargo and
   Cargo-compatible tools (`sccache`, shared `CARGO_TARGET_DIR`).

The target command surface:

```
rx init
rx run <task> [--affected] [-p <package>] [-- <args>]
rx ci [--affected]
rx graph
rx cache status|gc
rx doctor
```

plus thin aliases (`rx build`, `rx check`, `rx test`, `rx fmt`, `rx lint`)
that route through `rx run` — one task executor, one workspace model, one
process/output abstraction.

# Non-goals

rx is **not**:

- a Cargo replacement or wrapper for every Cargo subcommand
- a toolchain manager (that is rustup's job)
- a release/publishing tool
- a project scaffolding / template tool
- a dependency auditing, SBOM, or supply-chain security tool
- a private registry client
- a snapshot/fuzz/mutation testing framework
- a compiler cache (that is sccache's job; Cargo owns `target/`)
- a plugin platform or daemon

These are legitimate tools — as separate products. Any of them can be invoked
from a configured rx task.

# Feature freeze

Feature development outside the scope above is frozen. Commands and modules
outside the target surface will be deleted (not deprecated) in 0.2 — the
project is pre-1.0 precisely so this kind of correction stays cheap.

# Known correctness limits (why the artifact cache is opt-in)

The build artifact cache fingerprint currently covers: profile, rx-generated
`RUSTFLAGS`, `Cargo.toml`, `Cargo.lock`, and `.rs` files under `src/`. It does
**not** cover: target triple, package selection, feature flags, Rust toolchain
version, `build.rs`, `.cargo/config.toml`, compilation-relevant environment
variables, files pulled in by `include_bytes!`/`include_str!`, or sources
under `tests/`, `benches/`, and `examples/`.

Until the fingerprint is complete (or the cache is replaced by first-class
`sccache` integration), artifact restoration is **disabled by default**
(`build.cache = false`) and is never used for `--package` or `--target`
builds. Semantic-fingerprint build skipping (skipping rebuilds when a crate's
public API is unchanged) was removed entirely: it could skip rebuilding a
crate whose implementation changed.
