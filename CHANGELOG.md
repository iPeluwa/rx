# Changelog

All notable changes to rx will be documented in this file.

## [Unreleased]

### Added
- Remote/shared cache support (S3, GCS, local path) via `build.remote_cache` config
- GitHub Action (`action/action.yml`) for CI integration
- VS Code extension with 15 commands, task provider, and problem matchers
- MSRV compatibility checking (`rx compat`)
- Smart test orchestration with failure-based ordering and parallel sharding (`rx test-smart`)
- Build sandbox for detecting undeclared dependencies (`rx sandbox`)
- Private registry support (`rx registry login/list/add`)
- Lockfile policy enforcement (`rx lockfile check/enforce`)
- Opt-in anonymous telemetry (`rx telemetry on/off/status`)
- Persistent background workers (`rx worker warm/status/stop`)
- Background daemon with Unix socket IPC (`rx daemon start/stop/status/ping`)
- Pipelined workspace builds (check + build overlap)
- Semantic fingerprinting — only rebuild when public API changes
- Parallel cache restore with reflink/hardlink/copy fallback
- Incremental linking optimizations (`build.incremental_link` config)
- PGO (Profile-Guided Optimization) in release CI pipeline
- Watch mode JSON integration for richer error display in verbose mode
- 20 new error code hints (E0373, E0658, E0015, etc.)

### Fixed
- Clippy `manual_strip` warning in watch module
- Clippy `useless_format` warnings in cargo_output and sbom modules
- Dead code warnings for scaffolded pipeline module

## [0.1.0] - 2024-12-01

### Added
- Initial release
- Core build system with fast linker detection and caching
- Global content-addressed artifact cache with xxHash fingerprinting
- Workspace orchestration with dependency-aware parallel execution
- 40+ CLI commands covering the full Rust development workflow
- Project templates (axum, cli, wasm, lib)
- Native file watcher (no cargo-watch dependency)
- Shell completions with dynamic context-aware suggestions
- Plugin system
- Build statistics tracking
- Release automation
- Coverage reports
- Affected-only testing
- SBOM generation (SPDX, CycloneDX)
- Self-update mechanism
