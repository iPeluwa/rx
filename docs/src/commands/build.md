# Build & Run

The `rx build` and `rx run` commands compile your project with automatic fast linker detection, artifact caching, and cross-compilation support.

## Basic usage

```sh
rx build                        # debug build
rx build --release              # release build
rx run                          # build and run the binary
rx run -- --port 8080           # pass arguments to the binary
rx check                        # type-check only (no codegen)
```

## Fast linker

rx auto-detects the fastest available linker on your system:

1. **mold** -- fastest, preferred when available
2. **lld** -- fast, widely available
3. **system** -- default fallback (cc/ld)

The detected linker is cached at `~/.rx/env.lock` so detection only runs once. Override it in `rx.toml`:

```toml
[build]
linker = "mold"    # or "lld", "system", "auto"
```

## Cross-compilation

```sh
rx build --target aarch64-unknown-linux-gnu
rx build --target wasm32-unknown-unknown
rx build --target x86_64-pc-windows-msvc
```

rx passes the `--target` flag through to cargo. Ensure the target is installed:

```sh
rx toolchain install stable
rustup target add aarch64-unknown-linux-gnu
```

## Caching

When `cache = true` (the default), rx maintains a global content-addressed cache at `~/.rx/cache`:

1. **Mtime check** -- if no source file has changed, skip everything
2. **Fingerprint** -- compute xxh3-128 hash of sources, Cargo.toml, Cargo.lock, profile, and RUSTFLAGS
3. **Cache hit** -- restore artifacts via reflink/hardlink into `target/`
4. **Cache miss** -- build normally, then store artifacts for next time

Disable caching per-command or in config:

```sh
rx build --no-cache             # skip cache for this build
```

```toml
[build]
cache = false
```

## Incremental linking

When `incremental_link = true` (the default), rx enables:

- **split-debuginfo** -- keeps debug info separate from the binary
- **`--as-needed`** -- only links referenced libraries

This significantly reduces link times for iterative debug builds.

## RUSTFLAGS

Add extra RUSTFLAGS in `rx.toml`:

```toml
[build]
rustflags = ["-C", "target-cpu=native"]
```

These are appended to any existing RUSTFLAGS environment variable.

## Parallel jobs

```toml
[build]
jobs = 0    # auto-detect (default)
jobs = 4    # use exactly 4 parallel jobs
```

## Flags reference

| Flag | Description |
|------|-------------|
| `--release` | Build in release mode |
| `--target <triple>` | Cross-compile for a target |
| `--no-cache` | Skip artifact cache |
| `--verbose` | Show cache paths and timing |
| `--quiet` | Suppress non-error output |
| `--profile <name>` | Use a config profile |

## Related commands

- `rx check` -- type-check without building (faster feedback loop)
- `rx clean` -- remove build artifacts
- `rx clean --gc` -- clean and garbage-collect the global cache
