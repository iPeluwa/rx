# Introduction

**rx** is a fast local-CI and task runner for Rust workspaces. Define your pipeline once, run it the same way locally and in CI.

## The problem

CI failures you could have caught locally waste round-trips, and every project reinvents the same fmt/clippy/test/build pipeline in shell scripts and YAML. Workspace-aware concerns — which packages did my change affect? what order do tasks run in? — get solved ad hoc, per repo.

## What rx does

- **One-command CI** — `rx ci` runs your full pipeline locally
- **Task scripts** — define tasks once in `rx.toml`, run them anywhere with `rx script`
- **Affected-only testing** — `rx test --affected` tests only packages changed since a base ref
- **Workspace orchestration** — dependency-aware parallel execution across members
- **Unified commands** — `rx test` picks nextest when available, `rx lint` runs clippy strict, `rx fmt` runs rustfmt
- **Fast builds** — auto-detected `mold`/`lld` linkers
- **CI generation** — `rx init --ci` writes a workflow that mirrors your local pipeline

rx deliberately does **not** replace Cargo, rustup, or the cargo plugin ecosystem. Compilation scheduling and `target/` belong to Cargo; compiler caching belongs to tools like sccache; releases, audits, and scaffolding belong to their dedicated tools. See [PRODUCT.md](https://github.com/iPeluwa/rx/blob/master/PRODUCT.md) for the full list of non-goals.

## Getting started

```sh
curl -fsSL https://raw.githubusercontent.com/iPeluwa/rx/master/install.sh | sh
cd my-rust-project
rx init --ci
rx ci
```
