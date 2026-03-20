# Changelog

All notable changes to rx will be documented in this file.

## [Unreleased]

## [0.1.0] - 2025-03-20

### Added
- Core build system with fast linker detection (mold/lld auto-detect) and caching
- Global content-addressed artifact cache with xxHash fingerprinting
- Remote/shared cache support (S3, GCS, local path) via `build.remote_cache` config
- Semantic fingerprinting — only rebuild when public API changes
- Pipelined workspace builds with event-driven scheduler (check + build overlap)
- Workspace orchestration with dependency-aware parallel execution
- 50+ CLI commands covering the full Rust development workflow
- Smart test orchestration with failure-based ordering and parallel sharding (`rx test-smart`)
- Flaky test detection with automatic flip-flop tracking
- Build sandbox for detecting undeclared dependencies (`rx sandbox`)
- Background daemon with Unix socket IPC (`rx daemon start/stop/status/ping`)
- Persistent background workers (`rx worker warm/status/stop`)
- MSRV compatibility checking (`rx compat`)
- Private registry support (`rx registry login/list/add`)
- Lockfile policy enforcement (`rx lockfile check/enforce`)
- Opt-in anonymous telemetry with export/reporting (`rx telemetry on/off/status/export/report`)
- Self-update with SHA256 checksum verification (`rx self-update`)
- Workspace-level remote cache integration (`rx ws cache-push/cache-pull`)
- Incremental linking optimizations (`build.incremental_link` config)
- PGO (Profile-Guided Optimization) in release CI pipeline
- GitHub Action for CI integration (`action/action.yml`)
- VS Code extension with 15 commands, task provider, and problem matchers
- Project templates (axum, cli, wasm, lib)
- Native file watcher (no cargo-watch dependency)
- Shell completions (bash, zsh, fish, PowerShell, elvish)
- Man page generation (`rx manpage`)
- Plugin system
- Build statistics tracking
- Release automation (`rx release patch/minor/major`)
- Coverage reports (`rx coverage`)
- Affected-only testing (`rx test --affected`)
- SBOM generation (SPDX, CycloneDX)
- Project config with profiles, scripts, env vars (`rx.toml`)
- Cross-compilation support (`rx build --target <triple>`)
- 20+ error code hints for common Rust compiler errors
- mdBook documentation site
- GitHub Actions CI with MSRV and bench compile checks
