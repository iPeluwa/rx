# rx

A fast, unified Rust toolchain manager. One binary to replace the fragmented Rust CLI ecosystem.

## Why

The Rust toolchain is powerful but fragmented. You need `rustup`, `cargo`, `clippy`, `rustfmt`, `cargo-nextest`, `cargo-watch`, `sccache` — all installed separately, versioned independently, and configured in different places. Build times are slow, `target/` directories are massive, and workspace support is clunky.

**rx** wraps and extends Cargo into a single, opinionated CLI with:

- **Fast builds** — auto-detects `mold`/`lld` linkers, caches detection results persistently
- **Global artifact cache** — content-addressed store with xxHash fingerprinting, atomic writes, file locking, and mtime fast-path
- **Cross-compilation** — `rx build --target <triple>` for easy cross-compiling
- **Workspace orchestration** — dependency-aware parallel execution across workspace members
- **Unified commands** — `rx test` uses nextest when available, `rx lint` runs clippy with strict defaults, `rx fmt` runs rustfmt
- **One-command CI** — `rx ci` runs your full pipeline locally (fmt, clippy, test, build)
- **Auto-fix everything** — `rx fix` applies compiler suggestions, clippy fixes, and formatting in one step
- **Project config** — `rx.toml` with profiles, scripts, env vars, and config validation
- **Project templates** — `rx new --template axum/cli/wasm/lib` for opinionated scaffolding
- **Release automation** — `rx release patch` bumps version, commits, tags, and pushes
- **Native file watcher** — `rx watch` uses `notify` directly, no cargo-watch dependency
- **Coverage reports** — `rx coverage` with `--lcov` for CI and `--open` for local dev
- **Affected-only testing** — `rx test --affected` only tests packages changed since a base ref
- **Plugin system** — drop executables in `~/.rx/plugins/` and run them with `rx plugin run`
- **Build stats** — `rx stats show` tracks build time trends across sessions
- **Context-aware completions** — workspace members, installed targets, toolchains, and scripts
- **Colored output** — clear, color-coded status messages with timing and progress indicators
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
# Bash (includes dynamic completions for workspace members, targets, scripts)
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
rx new myproject               # new binary project
rx new myapi --template axum   # new axum web project
rx new mycli --template cli    # new clap CLI project
cd myproject
rx run
```

## Commands

| Command | Description |
|---|---|
| `rx init` | Generate `rx.toml` with smart defaults |
| `rx init --migrate` | Auto-detect project settings from existing tools |
| `rx init --ci` | Also generate `.github/workflows/ci.yml` |
| `rx config` | Show resolved configuration |
| `rx new <name>` | Create a new Rust project |
| `rx new <name> --template <t>` | Create from template: `axum`, `cli`, `wasm`, `lib` |
| `rx build` | Build with fast linker + caching |
| `rx build --target <triple>` | Cross-compile for a target triple |
| `rx run [-- args...]` | Build and run (args pass through to binary) |
| `rx check` | Type-check without building (fast feedback) |
| `rx test` | Run tests (nextest if available) |
| `rx test --affected` | Only test packages changed since base ref |
| `rx fmt` | Format code |
| `rx lint` | Lint with clippy |
| `rx fix` | Auto-fix everything (compiler + clippy + fmt) |
| `rx ci` | Run full CI pipeline locally |
| `rx bench` | Run benchmarks |
| `rx expand` | Expand macros (requires cargo-expand) |
| `rx publish` | Publish crate(s) to crates.io |
| `rx release <ver>` | Bump version, commit, tag, and push |
| `rx coverage` | Generate code coverage report |
| `rx size` | Show binary size (+ cargo-bloat breakdown) |
| `rx bloat` | Analyze binary bloat by function or crate |
| `rx tree` | Show dependency tree |
| `rx deps` | Dependency health dashboard (tree + outdated + audit) |
| `rx outdated` | Check for outdated dependencies |
| `rx audit` | Audit dependencies for security vulnerabilities |
| `rx doc` | Build documentation |
| `rx doctor` | Check your development environment |
| `rx upgrade` | Update toolchains and dependencies |
| `rx self-update` | Update rx to the latest version |
| `rx completions <shell>` | Generate shell completions |
| `rx script <name>` | Run a script defined in rx.toml |
| `rx pkg add/remove/upgrade/list` | Manage dependencies |
| `rx toolchain install/use/list/update` | Manage Rust toolchains |
| `rx cache status/gc/purge` | Manage the global artifact cache |
| `rx ws list/graph/run/script/exec` | Workspace orchestration |
| `rx watch` | Watch for changes and rebuild (native, no cargo-watch) |
| `rx clean` | Clean build artifacts |
| `rx clean --all` | Clean all workspace member target directories |
| `rx env show/shell` | Manage environment variables from rx.toml |
| `rx plugin list/run` | Manage and run plugins |
| `rx stats show/clear` | View or clear build time statistics |

### Global flags

| Flag | Description |
|---|---|
| `--quiet` / `-q` | Suppress non-error output |
| `--verbose` / `-v` | Show extra detail (cache paths, timing, etc.) |
| `--profile <name>` | Use a config profile (e.g. `--profile ci`) |

All commands support these flags. For example:

```sh
rx --quiet build --release    # silent build
rx --verbose test             # show timing and debug info
rx --profile ci test          # use CI profile overrides
```

## Configuration

Run `rx init` to generate an `rx.toml`. Smart defaults are applied based on your project — workspaces get a `ci` script, and if `mold` is available it's set as the default linker. Unknown keys in `rx.toml` produce a warning so typos don't silently fail.

Use `rx init --migrate` to auto-detect your project's existing tools (linkers, nextest, Makefiles, benchmarks, error handling crates) and generate a tailored config.

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
bench = "cargo bench"

[env]
RUST_BACKTRACE = "1"
```

### Config profiles

Override settings per context with `[profile.<name>]`:

```toml
[profile.ci]
build = { cache = false, jobs = 2 }
lint = { severity = "deny" }
test = { runner = "nextest" }
env = { CI = "true" }
```

Use with `rx --profile ci build`.

Config is resolved by merging `~/.rx/config.toml` (global) with the project's `rx.toml`. Project values override global.

## Project templates

Create new projects from opinionated templates:

```sh
rx new myapi --template axum     # Axum web API with tokio, serde, tracing
rx new mycli --template cli      # Clap CLI with anyhow error handling
rx new mywasm --template wasm    # wasm-bindgen library with tests
rx new mylib --template lib      # Library with doc tests and MIT/Apache-2.0
```

Each template creates a complete project with `Cargo.toml`, source files, `.gitignore`, and git init.

## Release automation

```sh
rx release patch                 # bump 0.1.0 → 0.1.1, commit, tag, push
rx release minor                 # bump 0.1.0 → 0.2.0
rx release major                 # bump 0.1.0 → 1.0.0
rx release 2.0.0                 # set explicit version
rx release patch --dry-run       # preview without changes
rx release patch --no-push       # commit and tag, but don't push
```

## Coverage

```sh
rx coverage                      # HTML report (uses cargo-llvm-cov or tarpaulin)
rx coverage --open               # build and open in browser
rx coverage --lcov               # LCOV output for CI (writes lcov.info)
```

## Affected-only testing

```sh
rx test --affected               # test packages changed since HEAD~1
rx test --affected --base main   # test packages changed since main branch
```

Maps changed files from `git diff` to workspace members and only runs tests for affected packages.

## Scripts

Define custom scripts in `rx.toml`:

```toml
[scripts]
ci = "cargo fmt --check && cargo clippy -- -D warnings && cargo test"
bench = "cargo bench"
deploy = "cargo build --release && scp target/release/myapp server:/opt/"
```

```sh
rx script ci          # run the "ci" script
rx script             # list all available scripts
rx ws script ci       # run "ci" across all workspace members
```

## Environment management

```sh
rx env show           # display resolved env vars from rx.toml
rx env shell          # spawn a subshell with env vars loaded
```

## Plugin system

Drop any executable named `rx-<name>` into `~/.rx/plugins/` (or anywhere on PATH):

```sh
rx plugin list        # list available plugins
rx plugin run myplugin -- --flag    # run a plugin with args
```

## Build stats

rx tracks build time statistics across sessions:

```sh
rx stats show         # per-command avg/min/max + recent history
rx stats clear        # clear all recorded stats
```

## Workflow commands

### rx check — fast type-checking

```sh
rx check                    # type-check the whole project
rx check --package mylib    # type-check a single package
```

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

Runs the full CI pipeline locally before pushing. If a `ci` script is defined in `rx.toml`, that's used. Otherwise the default pipeline runs: `fmt --check` → `clippy` → `test` → `build`.

### rx size — binary size analysis

```sh
rx size                # debug build size
rx size --release      # release build size
```

### rx bloat — binary bloat analysis

```sh
rx bloat               # function-level bloat breakdown
rx bloat --release     # release build
rx bloat --crates      # group by crate instead of function
```

### rx deps — dependency health dashboard

```sh
rx deps
```

Runs dependency tree, outdated check, and security audit in one command.

### rx doc — documentation builder

```sh
rx doc                 # build docs
rx doc --open          # build and open in browser
rx doc --no-deps       # skip dependency docs
rx doc --watch         # rebuild on changes (uses cargo-watch)
```

### rx tree — dependency tree

```sh
rx tree                        # full dependency tree
rx tree --duplicates           # show only duplicate dependencies
rx tree --depth 2              # limit tree depth
```

### rx watch — native file watcher

```sh
rx watch                       # watch and run default command (build)
rx watch --cmd "test"          # watch and run tests
```

Uses the `notify` crate directly — no need to install cargo-watch. Watches `src/`, `Cargo.toml`, `Cargo.lock`, `benches/`, `examples/`, `tests/`, and `build.rs`. Ignores `target/`, `.git/`, and patterns from `rx.toml`.

## Cache

rx maintains a global content-addressed artifact cache at `~/.rx/cache`. The cache is designed for correctness even under concurrent use:

1. An **mtime fast-path** checks if any source file has changed since the last build — if nothing changed, the cached fingerprint is reused instantly without reading file contents
2. On mtime mismatch, a full **xxHash (xxh3-128)** fingerprint is computed from `Cargo.toml`, `Cargo.lock`, all source files, the build profile, and RUSTFLAGS
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
rx clean --all     # clean all workspace member target/ directories
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

Members are grouped into parallel "waves" based on the dependency graph (Kahn's algorithm for topological sort). Independent packages build concurrently; dependent packages wait for their dependencies to complete.

## Publishing

`rx publish` handles workspace-aware publishing to crates.io:

```sh
rx publish                      # publish all workspace members in dependency order
rx publish --package mylib      # publish a single crate
rx publish --dry-run            # validate without publishing
```

## Doctor

`rx doctor` checks that your development environment is properly set up and refreshes the persistent env detection cache:

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
──────────────────────────────────────────────────
  Env cache: refreshed

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

## Performance

rx is built for speed:

- **xxHash (xxh3-128)** for fingerprinting — ~10x faster than SHA-256
- **Mtime fast-path** — skips content hashing entirely when no files have changed
- **Persistent env cache** — linker detection results cached to `~/.rx/env.lock`, refreshed by `rx doctor`
- **Native file watcher** — uses `notify` crate directly instead of spawning cargo-watch
- **Optimized release binary** — thin LTO, single codegen unit, stripped symbols, panic=abort
- **Lazy config loading** — commands that don't need config skip loading it
- **Hardlink cache restore** — artifacts hardlinked from cache instead of copied when possible

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
├── cli/            CLI definition (clap derive) with lazy config loading + profiles
├── config/         rx.toml parsing, global/project merge, profiles, validation
├── build/          cargo build with fast linker, cache, cross-compilation, env cache
├── cache/          content-addressed store with xxHash, atomic writes, file locking, mtime fast-path
├── cargo_output/   cargo JSON output parser for custom build rendering
├── workspace/      dependency graph, topo sort (Kahn's), parallel wave execution
├── output/         colored output, progress spinners, timing, verbosity control
├── watch/          native file watcher (notify crate), smart filtering
├── completions/    shell completions + context-aware dynamic completions
├── templates/      project templates (axum, cli, wasm, lib)
├── release/        version bumping, commit, tag, push automation
├── coverage/       code coverage (cargo-llvm-cov / tarpaulin, HTML + LCOV)
├── affected/       git-diff-based affected package detection
├── script/         rx.toml script runner
├── stats/          build time tracking and statistics
├── env/            environment variable management
├── plugin/         plugin discovery and execution
├── migrate/        auto-detection and config generation from existing projects
├── deps/           dependency health dashboard
├── bloat/          binary bloat analysis
├── doc/            documentation builder with --watch
├── check/          fast type-checking (cargo check)
├── fix/            auto-fix pipeline (cargo fix + clippy --fix + fmt)
├── ci/             local CI pipeline runner
├── size/           binary size analysis with cargo-bloat support
├── tree/           dependency tree visualization
├── outdated/       outdated dependency checker
├── audit/          security vulnerability auditing
├── selfupdate/     self-update mechanism
├── pkg/            dependency management (add/remove/upgrade)
├── toolchain/      rustup wrapper for toolchain management
├── test/           test runner with timing (auto-selects nextest)
├── fmt/            rustfmt wrapper with timing
├── lint/           clippy wrapper with configurable severity and timing
├── bench/          benchmark runner with timing
├── expand/         macro expansion (cargo-expand)
├── publish/        workspace-aware crates.io publishing with progress
├── doctor/         environment health checks + env cache refresh
├── upgrade/        toolchain and dependency updater with timing
└── install.sh      self-installer script
```

## Testing

rx has 92 tests across 5 test suites:

```sh
cargo test
```

| Suite | Tests | Coverage |
|---|---|---|
| `cache_tests` | 8 | Fingerprinting, cache hit/miss, store/restore |
| `cli_tests` | 55 | All CLI commands and flags parse correctly |
| `config_tests` | 8 | Config loading, merging, profiles, serialization |
| `integration_tests` | 10 | End-to-end: init, build, test, fmt, doctor, flags |
| `workspace_tests` | 11 | Topo sort, parallel waves, cycle detection |

CI runs on every push: check, test (ubuntu + macos), clippy, fmt, and MSRV verification.

## License

MIT
