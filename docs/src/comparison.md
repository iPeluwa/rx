# Comparison

How rx compares to other Rust build tools and task runners.

## rx vs raw Cargo

| Feature | Cargo | rx |
|---------|-------|----|
| Build, test, fmt, clippy | Yes (separate commands) | Yes (unified + `rx ci`) |
| Global artifact cache | No | Yes (content-addressed, xxHash) |
| Remote shared cache | No | Yes (S3, GCS, local path) |
| Semantic fingerprinting | No | Yes (only rebuild on API changes) |
| Fast linker detection | No | Yes (auto-detects mold/lld) |
| Workspace parallel waves | Basic | Dependency-aware with event-driven scheduler |
| Smart test ordering | No | Yes (failure-first, flaky detection) |
| Project templates | `cargo init` | `rx new --template axum/cli/wasm/lib` |
| Config file | Cargo.toml only | rx.toml with profiles, scripts, env |
| Background daemon | No | Yes (Unix socket IPC) |
| Shell completions | No | Yes (bash, zsh, fish, PowerShell) |
| MSRV verification | No | `rx compat` |
| SBOM generation | No | `rx sbom` (SPDX, CycloneDX) |

rx wraps Cargo — it doesn't replace it. Every rx command runs standard Cargo under the hood.

## rx vs cargo-make

[cargo-make](https://github.com/nickel-org/cargo-make) is a task runner with a Makefile.toml format.

| Feature | cargo-make | rx |
|---------|------------|----|
| Task definitions | Makefile.toml (verbose) | rx.toml `[scripts]` (concise) |
| Built-in Rust commands | No (shell tasks) | Yes (build, test, lint, fmt, etc.) |
| Caching | No | Global + remote cache |
| Workspace awareness | Plugin-based | Built-in with parallel waves |
| Installation | `cargo install cargo-make` | Single binary, no Cargo needed |

## rx vs just

[just](https://github.com/casey/just) is a command runner (like make but simpler).

| Feature | just | rx |
|---------|------|----|
| Purpose | General task runner | Rust-specific toolchain manager |
| Rust integration | None (runs shell commands) | Deep (understands Cargo, workspaces, targets) |
| Caching | No | Content-addressed artifact cache |
| Build optimization | No | Linker detection, PGO, pipelining |
| Config | justfile | rx.toml |

## rx vs cargo-xtask

[cargo-xtask](https://github.com/matklad/cargo-xtask) is a pattern for writing build scripts in Rust.

| Feature | cargo-xtask | rx |
|---------|-------------|----|
| Setup | Write Rust code per-project | Zero config, works out of the box |
| Maintenance | You maintain the xtask crate | rx maintains the tooling |
| Caching | No | Built-in |
| Cross-project reuse | Copy-paste | Same rx binary everywhere |

## rx vs sccache

[sccache](https://github.com/mozilla/sccache) is a shared compilation cache.

| Feature | sccache | rx |
|---------|----------|----|
| Scope | Compilation cache only | Full toolchain manager + cache |
| Cache granularity | Per-compilation-unit | Per-crate with semantic fingerprinting |
| Remote backends | S3, GCS, Azure, Redis | S3, GCS, local path |
| Additional features | None | 50+ commands, workspace orchestration, etc. |

rx and sccache can be used together — they operate at different levels.

## Summary

Use **rx** if you want a single tool that handles your entire Rust development workflow with built-in caching, smart builds, and zero configuration. Use the other tools if you need a general-purpose task runner (just, cargo-make) or only need compilation caching (sccache).
