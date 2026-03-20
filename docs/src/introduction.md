# Introduction

**rx** is a fast, unified Rust toolchain manager. One binary to replace the fragmented Rust CLI ecosystem.

The Rust toolchain is powerful but fragmented. You need `rustup`, `cargo`, `clippy`, `rustfmt`, `cargo-nextest`, `cargo-watch`, `sccache` — all installed separately, versioned independently, and configured in different places.

rx wraps and extends Cargo into a single, opinionated CLI with 50+ commands covering the full Rust development workflow.

## Key features

- **Fast builds** with auto-detected linkers and content-addressed caching
- **Workspace orchestration** with dependency-aware parallel execution
- **Smart testing** with failure-based ordering and parallel sharding
- **Remote cache** for sharing build artifacts across CI runners
- **Semantic fingerprinting** — only rebuild when public API changes
- **Background daemon** for instant command startup
- **Project config** via `rx.toml` with profiles and scripts
- **50+ commands** from build to release to audit

## Getting started

```sh
curl -fsSL https://raw.githubusercontent.com/iPeluwa/rx/master/install.sh | sh
rx new myproject
cd myproject
rx run
```
