# rx

A fast, unified Rust toolchain manager. One binary to replace the fragmented Rust CLI ecosystem.

## Why

The Rust toolchain is powerful but fragmented. You need `rustup`, `cargo`, `clippy`, `rustfmt`, `cargo-nextest`, `cargo-watch`, `sccache` — all installed separately, versioned independently, and configured in different places. Build times are slow, `target/` directories are massive, and workspace support is clunky.

**rx** wraps and extends Cargo into a single, opinionated CLI with:

- **Fast builds** — auto-detects and uses `mold` or `lld` linkers
- **Global artifact cache** — content-addressed store at `~/.rx/cache` with mtime-based fast-path invalidation
- **Cross-compilation** — `rx build --target <triple>` for easy cross-compiling
- **Workspace orchestration** — dependency-aware parallel execution across workspace members
- **Unified commands** — `rx test` uses nextest when available, `rx lint` runs clippy with strict defaults, `rx fmt` runs rustfmt
- **Project config** — `rx.toml` controls build, test, lint, fmt, watch, scripts, and env vars
- **Colored output** — clear, color-coded status messages throughout
- **Environment checks** — `rx doctor` verifies your toolchain is ready

## Install

### One-liner

```sh
curl -fsSL https://raw.githubusercontent.com/iPeluwa/rx/master/install.sh | sh
```

This downloads a prebuilt binary for your platform, or falls back to `cargo install` from source.

### From source

```sh
cargo install --path .
```

### Shell completions

```sh
# Bash
rx completions bash >> ~/.bashrc

# Zsh
rx completions zsh >> ~/.zshrc

# Fish
rx completions fish > ~/.config/fish/completions/rx.fish

# PowerShell
rx completions powershell >> $PROFILE
```

## Quick start

```sh
rx new myproject
cd myproject
rx run
```

## Commands

| Command | Description |
|---|---|
| `rx init` | Generate `rx.toml` with defaults |
| `rx config` | Show resolved configuration |
| `rx new <name>` | Create a new Rust project |
| `rx build` | Build with fast linker + caching |
| `rx build --target <triple>` | Cross-compile for a target triple |
| `rx run` | Build and run |
| `rx test` | Run tests (nextest if available) |
| `rx fmt` | Format code |
| `rx lint` | Lint with clippy |
| `rx bench` | Run benchmarks |
| `rx expand` | Expand macros (requires cargo-expand) |
| `rx publish` | Publish crate(s) to crates.io |
| `rx doctor` | Check your development environment |
| `rx upgrade` | Update toolchains and dependencies |
| `rx completions <shell>` | Generate shell completions |
| `rx pkg add/remove/upgrade/list` | Manage dependencies |
| `rx toolchain install/use/list/update` | Manage Rust toolchains |
| `rx cache status/gc/purge` | Manage the global artifact cache |
| `rx ws list/graph/run/script/exec` | Workspace orchestration |
| `rx watch` | Watch for changes and rebuild |
| `rx clean` | Clean build artifacts |

## Configuration

Run `rx init` to generate an `rx.toml`:

```toml
[build]
linker = "auto"       # "auto", "mold", "lld", or "system"
rustflags = []        # extra RUSTFLAGS
cache = true          # enable global artifact cache
jobs = 0              # parallel jobs (0 = auto)

[test]
runner = "auto"       # "auto", "nextest", or "cargo"
extra_args = []

[lint]
severity = "deny"     # "deny", "warn", or "allow"
extra_lints = []      # e.g. ["clippy::pedantic"]

[fmt]
extra_args = []

[watch]
cmd = "build"         # default command on file changes
ignore = []           # file patterns to ignore

[scripts]
ci = "cargo fmt --check && cargo clippy && cargo test"

[env]
RUST_BACKTRACE = "1"
```

Config is resolved by merging `~/.rx/config.toml` (global) with the project's `rx.toml`. Project values override global.

## Cache

rx maintains a global content-addressed artifact cache at `~/.rx/cache`. On each build:

1. An **mtime fast-path** checks if any source file has changed since the last build — if nothing changed, the cached fingerprint is reused instantly without reading file contents
2. On mtime mismatch, a full SHA-256 fingerprint is computed from `Cargo.toml`, `Cargo.lock`, all source files, the build profile, and RUSTFLAGS
3. If a cached build matches the fingerprint, artifacts are hardlinked back into `target/` — skipping `cargo build` entirely
4. On cache miss, the build runs normally and results are stored for future use

```sh
rx cache status    # show cache size and artifact count
rx cache gc        # remove artifacts older than 30 days
rx cache purge     # delete the entire cache
rx clean --gc      # clean local target/ and GC global cache
```

## Workspace orchestration

For Cargo workspaces, `rx ws` provides dependency-aware execution:

```sh
rx ws list                  # list all workspace members
rx ws graph                 # show dependency graph
rx ws run build             # build all members in parallel waves
rx ws run test --release    # test all members in release mode
rx ws script ci             # run "ci" script from each member's rx.toml
rx ws exec "wc -l src/*.rs" # run a shell command in each member directory
```

Members are grouped into parallel "waves" based on the dependency graph. Independent packages build concurrently; dependent packages wait for their dependencies to complete.

## Publishing

`rx publish` handles workspace-aware publishing to crates.io:

```sh
rx publish                      # publish all workspace members in dependency order
rx publish --package mylib      # publish a single crate
rx publish --dry-run            # validate without publishing
```

When publishing an entire workspace, members are published in topological order with a delay between each to allow crates.io to index dependencies.

## Doctor

`rx doctor` checks that your development environment is properly set up:

```sh
$ rx doctor
rx doctor
──────────────────────────────────────────────────
  OK       rustc          (rustc 1.82.0)
  OK       cargo          (cargo 1.82.0)
  OK       rustup         (rustup 1.27.1)
  OK       rustfmt        (rustfmt 1.7.1)
  OK       clippy         (clippy 0.1.82)
  MISSING  mold           -> https://github.com/rui314/mold (optional)
  OK       lld            (LLD 18.1.8)
  OK       nextest        (cargo-nextest 0.9.72)
  OK       cargo-watch    (cargo-watch 8.5.2)
──────────────────────────────────────────────────

All required tools present.
```

## Architecture

```
rx (single binary)
├── cli/           CLI definition (clap derive)
├── config/        rx.toml parsing, global/project merge
├── build/         cargo build with fast linker, cache, cross-compilation
├── cache/         content-addressed artifact store with mtime fast-path + GC
├── workspace/     dependency graph, topo sort, parallel wave execution
├── output/        colored terminal output (owo-colors)
├── pkg/           dependency management (add/remove/upgrade)
├── toolchain/     rustup wrapper for toolchain management
├── test/          test runner (auto-selects nextest)
├── fmt/           rustfmt wrapper
├── lint/          clippy wrapper with configurable severity
├── watch/         cargo-watch wrapper with ignore patterns
├── bench/         benchmark runner
├── expand/        macro expansion (cargo-expand)
├── publish/       workspace-aware crates.io publishing
├── doctor/        environment health checks
├── upgrade/       toolchain and dependency updater
├── completions/   shell completion generator
└── install.sh     self-installer script
```

## License

MIT
