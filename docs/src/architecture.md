# Architecture

rx is a single Rust binary (MSRV 1.85.0) organized into focused modules. This page describes the high-level design.

## Module layout

```
rx
├── cli/               CLI definition (clap derive), lazy config loading, profiles
├── config/            rx.toml parsing, global/project merge, profile resolution, validation
├── build/             cargo build orchestration, fast linker, cross-compilation
├── cache/             Content-addressed artifact store (xxHash, atomic writes, reflink)
├── remote_cache/      S3/GCS/local shared cache backend
├── semantic_hash/     syn-based public API fingerprinting
├── pipeline/          Pipelined workspace builds (check + build overlap)
├── cargo_output/      Cargo JSON output parser with error hints
├── workspace/         Dependency graph, topological sort (Kahn's), parallel waves
├── daemon/            Background daemon with Unix socket IPC
├── worker/            Persistent warm check/fmt/lint processes
├── output/            Colored output, progress spinners, timing, verbosity
├── watch/             Native file watcher (notify crate), smart filtering
├── completions/       Shell completions with context-aware dynamic values
├── templates/         Project templates (axum, cli, wasm, lib)
├── release/           Version bumping, commit, tag, push automation
├── coverage/          Code coverage (cargo-llvm-cov / tarpaulin)
├── affected/          Git-diff-based affected package detection
├── test_orchestrator/ Smart test ordering, sharding, flaky detection
├── compat/            MSRV compatibility checking
├── sandbox/           Isolated builds (env-stripped)
├── registry/          Private crate registry configuration and auth
├── lockfile/          Lockfile health checking and CI enforcement
├── telemetry/         Opt-in anonymous usage analytics (local only)
├── script/            rx.toml script runner
├── stats/             Build time tracking and statistics
├── env/               Environment variable management
├── plugin/            Plugin discovery and execution (~/.rx/plugins/)
├── migrate/           Auto-detection of existing project settings
├── deps/              Dependency health dashboard
├── bloat/             Binary bloat analysis
├── hints/             25+ error code explanations with practical fixes
├── doc/               Documentation builder
├── sbom/              SPDX and CycloneDX bill of materials
├── test_advanced/     Snapshot, fuzz, and mutation testing
└── install.sh         Self-installer script
```

## How a command runs

1. **CLI parsing** -- `clap` parses arguments and flags. The `--profile` flag is captured as a global option.
2. **Config loading** (lazy) -- if the command needs configuration, rx loads `~/.rx/config.toml` and `./rx.toml`, merges them (project overrides global), and applies the active profile.
3. **Environment setup** -- environment variables from `[env]` are set. The cached env at `~/.rx/env.lock` provides linker paths and toolchain info.
4. **Command execution** -- the command module runs, calling cargo or other tools as subprocesses. Output is parsed from cargo's JSON stream for error hints and progress display.
5. **Cache update** -- if caching is enabled, the fingerprint and artifacts are stored in `~/.rx/cache`.

## Config resolution

Configuration is resolved in layers:

```
defaults  <  ~/.rx/config.toml  <  ./rx.toml  <  [profile.<name>]  <  CLI flags
```

Each layer overrides the previous. Unknown keys produce warnings.

## Cache design

The global cache at `~/.rx/cache` is content-addressed:

```
~/.rx/cache/
├── index.json          # fingerprint -> artifact metadata
├── mtime-snapshot.json # per-file modification times
├── lock                # file lock for concurrent access
└── artifacts/
    └── <fingerprint>/  # one directory per unique build
        ├── deps/
        ├── build/
        └── ...
```

**Correctness guarantees:**

- **Atomic writes** -- the index and mtime snapshots are written to a temp file and atomically renamed
- **File locking** -- a lock file prevents concurrent rx processes from corrupting the index
- **Staging directory** -- new artifacts are written to a staging directory, then renamed into place
- **Parallel I/O** -- rayon is used for parallel file copies during store/restore

## Workspace execution model

For workspace commands, rx builds a dependency graph using Cargo metadata and performs a topological sort (Kahn's algorithm). Packages are grouped into parallel "waves":

```
Wave 1: [core, utils]       # no dependencies, run in parallel
Wave 2: [api, cli]          # depend on wave 1, wait then run in parallel
Wave 3: [integration-tests] # depends on wave 2
```

Semantic fingerprinting allows skipping entire waves when the public API of their dependencies has not changed.

## Error handling

rx wraps all errors with context using `anyhow`. Every user-facing error includes:

- A clear description of what went wrong
- The underlying cause (if available)
- A hint on how to fix it (25+ error codes with practical explanations)

Run `rx explain <code>` to get a detailed explanation of any error code.
