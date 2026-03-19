# rx

A fast, unified Rust toolchain manager. One binary to replace the fragmented Rust CLI ecosystem.

## Why

The Rust toolchain is powerful but fragmented. You need `rustup`, `cargo`, `clippy`, `rustfmt`, `cargo-nextest`, `cargo-watch`, `sccache` — all installed separately, versioned independently, and configured in different places. Build times are slow, `target/` directories are massive, and workspace support is clunky.

**rx** wraps and extends Cargo into a single, opinionated CLI with:

- **Fast builds** — auto-detects and uses `mold` or `lld` linkers
- **Global artifact cache** — content-addressed store with atomic writes, file locking, and mtime-based fast-path invalidation
- **Cross-compilation** — `rx build --target <triple>` for easy cross-compiling
- **Workspace orchestration** — dependency-aware parallel execution across workspace members
- **Unified commands** — `rx test` uses nextest when available, `rx lint` runs clippy with strict defaults, `rx fmt` runs rustfmt
- **One-command CI** — `rx ci` runs your full pipeline locally (fmt, clippy, test, build)
- **Auto-fix everything** — `rx fix` applies compiler suggestions, clippy fixes, and formatting in one step
- **Project config** — `rx.toml` controls build, test, lint, fmt, watch, scripts, and env vars with validation
- **Colored output** — clear, color-coded status messages with timing and progress indicators
- **Environment checks** — `rx doctor` verifies your toolchain is ready
- **Dependency health** — `rx outdated`, `rx audit`, `rx tree` for full dependency visibility
- **Binary size analysis** — `rx size` shows binary size with optional cargo-bloat breakdown
- **Graceful signals** — Ctrl+C handling for clean shutdown
- **Actionable errors** — every failure includes hints on how to fix it
- **Lazy config loading** — commands that don't need config skip loading it for faster startup
- **Self-updating** — `rx self-update` updates rx to the latest version

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
| `rx init` | Generate `rx.toml` with smart defaults |
| `rx config` | Show resolved configuration |
| `rx new <name>` | Create a new Rust project |
| `rx build` | Build with fast linker + caching |
| `rx build --target <triple>` | Cross-compile for a target triple |
| `rx run [-- args...]` | Build and run (args pass through to binary) |
| `rx check` | Type-check without building (fast feedback) |
| `rx test` | Run tests (nextest if available) |
| `rx fmt` | Format code |
| `rx lint` | Lint with clippy |
| `rx fix` | Auto-fix everything (compiler + clippy + fmt) |
| `rx ci` | Run full CI pipeline locally |
| `rx bench` | Run benchmarks |
| `rx expand` | Expand macros (requires cargo-expand) |
| `rx publish` | Publish crate(s) to crates.io |
| `rx size` | Show binary size (+ cargo-bloat breakdown) |
| `rx tree` | Show dependency tree |
| `rx outdated` | Check for outdated dependencies |
| `rx audit` | Audit dependencies for security vulnerabilities |
| `rx doctor` | Check your development environment |
| `rx upgrade` | Update toolchains and dependencies |
| `rx self-update` | Update rx to the latest version |
| `rx completions <shell>` | Generate shell completions |
| `rx pkg add/remove/upgrade/list` | Manage dependencies |
| `rx toolchain install/use/list/update` | Manage Rust toolchains |
| `rx cache status/gc/purge` | Manage the global artifact cache |
| `rx ws list/graph/run/script/exec` | Workspace orchestration |
| `rx watch` | Watch for changes and rebuild |
| `rx clean` | Clean build artifacts |

### Global flags

| Flag | Description |
|---|---|
| `--quiet` / `-q` | Suppress non-error output |
| `--verbose` / `-v` | Show extra detail (cache paths, timing, etc.) |

All commands support these flags. For example:

```sh
rx --quiet build --release    # silent build
rx --verbose test             # show timing and debug info
```

## Configuration

Run `rx init` to generate an `rx.toml`. Smart defaults are applied based on your project — workspaces get a `ci` script, and if `mold` is available it's set as the default linker. Unknown keys in `rx.toml` produce a warning so typos don't silently fail.

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
ci = "cargo fmt --check && cargo clippy -- -D warnings && cargo test"

[env]
RUST_BACKTRACE = "1"
```

Config is resolved by merging `~/.rx/config.toml` (global) with the project's `rx.toml`. Project values override global.

## Workflow commands

### rx check — fast type-checking

```sh
rx check                    # type-check the whole project
rx check --package mylib    # type-check a single package
```

Runs `cargo check` with your configured linker and job count. Faster than a full build when you just want to verify your code compiles.

### rx fix — auto-fix everything

```sh
rx fix
```

Applies fixes in three passes:
1. **Compiler suggestions** — `cargo fix` for edition migrations, unused imports, etc.
2. **Clippy fixes** — `cargo clippy --fix` with your configured lint severity
3. **Formatting** — `cargo fmt` to clean up any remaining style issues

### rx ci — local CI pipeline

```sh
rx ci
```

Runs the full CI pipeline locally before pushing. If a `ci` script is defined in `rx.toml`, that's used. Otherwise the default pipeline runs: `fmt --check` → `clippy` → `test` → `build`. Fails fast on the first error with a clear message about which step failed.

### rx size — binary size analysis

```sh
rx size                # debug build size
rx size --release      # release build size
```

Builds the project and reports the binary size. If `cargo-bloat` is installed, also shows a breakdown of the top crate contributions by size.

### rx tree — dependency tree

```sh
rx tree                        # full dependency tree
rx tree --duplicates           # show only duplicate dependencies
rx tree --depth 2              # limit tree depth
rx tree --duplicates --depth 3 # combine flags
```

### rx outdated — check for updates

```sh
rx outdated
```

Uses `cargo-outdated` for a detailed report if installed, otherwise falls back to `cargo update --dry-run` to show what would change.

### rx audit — security vulnerabilities

```sh
rx audit
```

Runs `cargo-audit` to check all dependencies against the RustSec advisory database. Requires `cargo install cargo-audit`.

### rx self-update

```sh
rx self-update
```

Updates rx to the latest release. Uses the install script via `curl` when available, falls back to `cargo install --git`.

## Cache

rx maintains a global content-addressed artifact cache at `~/.rx/cache`. The cache is designed for correctness even under concurrent use:

1. An **mtime fast-path** checks if any source file has changed since the last build — if nothing changed, the cached fingerprint is reused instantly without reading file contents
2. On mtime mismatch, a full SHA-256 fingerprint is computed from `Cargo.toml`, `Cargo.lock`, all source files, the build profile, and RUSTFLAGS
3. If a cached build matches the fingerprint, artifacts are hardlinked back into `target/` — skipping `cargo build` entirely
4. On cache miss, the build runs normally and results are stored for future use

**Integrity guarantees:**
- **Atomic writes** — cache index and mtime snapshots are written to a temp file then atomically renamed, so interrupted writes can't corrupt state
- **File locking** — a lock file prevents concurrent `rx` processes from racing on the cache index
- **Staging directory** — new artifacts are written to a staging dir then renamed into place, so partial builds never pollute the cache

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

When publishing an entire workspace, members are published in topological order with a progress spinner while waiting for crates.io to index each dependency.

## Doctor

`rx doctor` checks that your development environment is properly set up:

```
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

## Error handling

rx is designed to give you actionable feedback when things go wrong:

```
$ rx build
[rx] could not find Cargo.toml in any parent directory
hint: run this command from inside a Rust project, or use `rx new <name>` to create one

$ rx lint
[rx] lint failed — run `rx lint --fix` to auto-fix what's possible

$ rx publish
[rx] publish failed for mylib
hint: check that you're logged in with `cargo login` and the package version is bumped
```

Every error includes context about what went wrong and a suggestion for how to fix it. Use `--verbose` for additional diagnostic detail.

## Releasing

rx includes a GitHub Actions workflow that automatically builds and publishes binaries when you push a version tag:

```sh
git tag v0.1.0
git push origin v0.1.0
```

This builds for four targets and attaches the binaries to a GitHub Release:
- `x86_64-unknown-linux-gnu`
- `aarch64-unknown-linux-gnu`
- `x86_64-apple-darwin`
- `aarch64-apple-darwin`

## Architecture

```
rx (single binary, MSRV 1.85.0)
├── cli/           CLI definition (clap derive) with lazy config loading
├── config/        rx.toml parsing, global/project merge, smart init, key validation
├── build/         cargo build with fast linker, cache, cross-compilation, timing
├── cache/         content-addressed store with atomic writes, file locking, mtime fast-path
├── workspace/     dependency graph, topo sort, parallel wave execution
├── output/        colored output, progress spinners, timing, verbosity control
├── check/         fast type-checking (cargo check)
├── fix/           auto-fix pipeline (cargo fix + clippy --fix + fmt)
├── ci/            local CI pipeline runner
├── size/          binary size analysis with cargo-bloat support
├── tree/          dependency tree visualization
├── outdated/      outdated dependency checker
├── audit/         security vulnerability auditing
├── selfupdate/    self-update mechanism
├── pkg/           dependency management (add/remove/upgrade)
├── toolchain/     rustup wrapper for toolchain management
├── test/          test runner with timing (auto-selects nextest)
├── fmt/           rustfmt wrapper with timing
├── lint/          clippy wrapper with configurable severity and timing
├── watch/         cargo-watch wrapper with ignore patterns
├── bench/         benchmark runner with timing
├── expand/        macro expansion (cargo-expand)
├── publish/       workspace-aware crates.io publishing with progress
├── doctor/        environment health checks
├── upgrade/       toolchain and dependency updater with timing
├── completions/   shell completion + man page generation
└── install.sh     self-installer script
```

## Testing

rx has 70 tests across 5 test suites:

```sh
cargo test
```

| Suite | Tests | Coverage |
|---|---|---|
| `cache_tests` | 8 | Fingerprinting, cache hit/miss, store/restore |
| `cli_tests` | 33 | All CLI commands parse correctly (including check, fix, ci, size, tree, outdated, audit, self-update) |
| `config_tests` | 8 | Config loading, merging, serialization |
| `integration_tests` | 10 | End-to-end: init, build, test, fmt, doctor, flags |
| `workspace_tests` | 11 | Topo sort, parallel waves, cycle detection |

CI runs on every push: check, test (ubuntu + macos), clippy, fmt, and MSRV verification.

## License

MIT
