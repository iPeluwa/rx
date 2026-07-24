# Comparison

How rx compares to other Rust build tools and task runners.

## rx vs raw Cargo

| Feature | Cargo | rx |
|---------|-------|----|
| Build, test, fmt, clippy | Yes (separate commands) | Yes (unified + `rx ci`) |
| One-command local CI | No | `rx ci` |
| Affected-package detection | No | `rx test --affected` |
| Workspace task orchestration | Basic (`--workspace`) | Dependency-aware parallel waves |
| Fast linker detection | No | Yes (auto-detects mold/lld) |
| Config file | Cargo.toml only | rx.toml with profiles, tasks, env |
| Shell completions | No | Yes (bash, zsh, fish, PowerShell) |

rx wraps Cargo — it doesn't replace it. Every rx command runs standard Cargo under the hood, and Cargo owns compilation scheduling and `target/`.

## rx vs cargo-make

[cargo-make](https://github.com/sagiegurari/cargo-make) is a task runner with a Makefile.toml format.

| Feature | cargo-make | rx |
|---------|------------|----|
| Task definitions | Makefile.toml (verbose) | rx.toml `[tasks]` with depends-on (concise) |
| Built-in Rust commands | No (shell tasks) | Yes (build, test, lint, fmt, ci) |
| Workspace awareness | Plugin-based | Built-in with parallel waves |
| Affected-package detection | No | Built-in |

## rx vs just

[just](https://github.com/casey/just) is a command runner (like make but simpler).

| Feature | just | rx |
|---------|------|----|
| Purpose | General task runner | Rust-specific local CI / task runner |
| Rust integration | None (runs shell commands) | Deep (understands Cargo workspaces) |
| Affected-package detection | No | Built-in |
| Config | justfile | rx.toml |

## rx vs cargo-xtask

[cargo-xtask](https://github.com/matklad/cargo-xtask) is a pattern for writing build scripts in Rust.

| Feature | cargo-xtask | rx |
|---------|-------------|----|
| Setup | Write Rust code per-project | Zero config, works out of the box |
| Maintenance | You maintain the xtask crate | rx maintains the tooling |
| Cross-project reuse | Copy-paste | Same rx binary everywhere |

## rx vs sccache

[sccache](https://github.com/mozilla/sccache) is a shared compilation cache. They are complementary, not competing: rx orchestrates tasks; sccache caches compilation. The Cargo documentation recommends sccache for sharing compiled dependencies, and rx defers compiler-level caching to it.

## Summary

Use **rx** if you want your CI pipeline to be defined once and runnable identically on your machine and in CI, with workspace-aware task execution and affected-package detection. Use a general-purpose task runner (just, cargo-make) if your tasks aren't Rust-shaped, and sccache for compilation caching either way.
