# Advanced Features

rx includes several advanced features for optimizing builds, improving test reliability, and integrating with team infrastructure.

## Build optimization

- [Remote Cache](./advanced/remote-cache.md) -- share build artifacts across CI runners and developers via S3, GCS, or a shared path
- [Semantic Fingerprinting](./advanced/semantic-fingerprint.md) -- skip downstream rebuilds when only private code changes
- [Build Sandbox](./advanced/sandbox.md) -- detect undeclared dependencies with isolated builds

## Test orchestration

- [Smart Test Orchestration](./advanced/test-orchestration.md) -- failure-based ordering, sharding, and flaky detection

## Compatibility

- [MSRV Compatibility](./advanced/msrv.md) -- verify code and dependencies work on your declared minimum Rust version

## Background services

- [Background Daemon](./advanced/daemon.md) -- persistent process for instant command startup
- [Persistent Workers](./advanced/workers.md) -- warm check/fmt/lint processes

## Infrastructure

- [Private Registries](./advanced/registries.md) -- configure and authenticate with private crate registries
- [Lockfile Policy](./advanced/lockfile.md) -- enforce lockfile health in CI
- [Telemetry](./advanced/telemetry.md) -- opt-in, local-only usage tracking
