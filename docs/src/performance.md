# Performance

rx is designed to minimize build and iteration time at every layer. This page describes each performance feature and how it works.

## xxHash fingerprinting

rx uses **xxHash (xxh3-128)** for all content hashing -- fingerprinting source files, cache keys, and artifact identity. xxh3-128 is approximately 10x faster than SHA-256 while providing sufficient collision resistance for build caching.

The fingerprint covers:

- `Cargo.toml` and `Cargo.lock` contents
- All source files in the crate
- The build profile (debug/release)
- Active RUSTFLAGS

## Semantic fingerprinting

Instead of rebuilding downstream crates whenever an upstream crate changes, rx parses Rust source files with `syn` and extracts only public API signatures (functions, structs, enums, traits, type aliases, impls). The fingerprint is computed from this extracted API surface.

This means changes to comments, formatting, private functions, or function bodies do not trigger downstream rebuilds. See [Semantic Fingerprinting](./advanced/semantic-fingerprint.md) for details.

## Mtime fast-path

Before computing any content hash, rx checks the `mtime` (modification time) of every source file against a stored snapshot. If no file has a newer mtime than the last recorded build, the cached fingerprint is reused instantly without reading any file contents.

This makes repeated `rx build` invocations with no changes effectively free.

## Reflink copy-on-write

When restoring cached artifacts into `target/`, rx uses **reflink** (copy-on-write) on filesystems that support it (APFS on macOS, btrfs on Linux). A reflink creates an instant zero-copy clone that only allocates new disk blocks when modified.

Fallback order:

1. Reflink (instant, zero-copy)
2. Hardlink (instant, shares inode)
3. Regular copy (byte-by-byte, always works)

## Parallel cache operations

Cache store and restore operations use `rayon` for parallel file copies. When storing or restoring many artifacts, multiple files are processed concurrently across all available CPU cores.

## Pipelined builds

In workspace builds, rx overlaps type-checking of downstream crates with code generation of upstream crates. Instead of waiting for a full build of a dependency before starting the dependent crate, downstream `cargo check` begins as soon as metadata is available.

This is especially effective in large workspaces where independent packages can make progress in parallel.

## Fast linker detection

rx auto-detects `mold` and `lld` linkers at first run and caches the detection result persistently at `~/.rx/env.lock`. Subsequent runs skip the detection entirely. Run `rx doctor` to refresh the cached environment.

The linker priority is:

1. `mold` (fastest)
2. `lld` (fast)
3. System linker (default fallback)

## Incremental linking

When `incremental_link = true` (the default), rx configures:

- **split-debuginfo** -- separates debug info from the binary, reducing link input size
- **`--as-needed`** -- only links libraries actually referenced by the binary

These reduce link times for iterative debug builds.

## PGO release builds

The rx binary itself is built with Profile-Guided Optimization (PGO) in CI:

1. Build an instrumented binary
2. Run the test suite to generate profile data
3. Rebuild with the profile data for optimized branch prediction and inlining

## Optimized release binary

The distributed rx binary is compiled with:

- Thin LTO (link-time optimization across crates)
- Single codegen unit (maximum optimization)
- Stripped symbols (smaller binary)
- `panic = abort` (no unwind tables)

## Native file watcher

`rx watch` uses the `notify` crate directly instead of spawning `cargo-watch` as a subprocess. This avoids process startup overhead and provides tighter integration with rx's file filtering and debouncing.

## Lazy config loading

Commands that do not need project configuration (like `rx self-update` or `rx completions`) skip loading and parsing `rx.toml` entirely, reducing startup latency.

## Persistent env cache

Linker detection, toolchain paths, and other environment probes are cached at `~/.rx/env.lock`. This avoids repeated `which mold`, `which lld`, and similar subprocess calls on every invocation.
