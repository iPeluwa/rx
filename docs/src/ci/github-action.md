# GitHub Action

rx provides an official GitHub Action at `iPeluwa/rx@v1` for easy CI integration.

## Basic usage

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: ci
```

This installs rx, sets up the Rust toolchain, caches build artifacts, and runs `rx ci`.

## Inputs

| Input | Default | Description |
|-------|---------|-------------|
| `version` | `latest` | rx version to install. Use `latest` for the newest release or a specific version like `0.5.0`. |
| `command` | `ci` | The rx command to run. Can be any valid rx command, e.g. `build --release`, `test`, `lint`. |
| `working-directory` | `.` | Working directory for rx commands. Set this if your Rust project is in a subdirectory. |
| `profile` | (empty) | rx config profile to use. Maps to the `--profile` flag. |
| `cache` | `true` | Enable build artifact caching. Caches the Cargo registry, git checkouts, rx cache, and `target/` directory. |
| `rust-toolchain` | `stable` | Rust toolchain to install via `rustup`. Examples: `stable`, `nightly`, `1.75.0`. |

## Examples

### Full CI pipeline

```yaml
name: CI
on: [push, pull_request]

jobs:
  ci:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iPeluwa/rx@v1
        with:
          command: ci
```

### Multiple steps

```yaml
jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: iPeluwa/rx@v1
        with:
          command: fmt --check
      - uses: iPeluwa/rx@v1
        with:
          command: lint
      - uses: iPeluwa/rx@v1
        with:
          command: test
```

### With CI profile

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: ci
    profile: ci
```

### Cross-platform matrix

```yaml
jobs:
  test:
    strategy:
      matrix:
        os: [ubuntu-latest, macos-latest, windows-latest]
    runs-on: ${{ matrix.os }}
    steps:
      - uses: actions/checkout@v4
      - uses: iPeluwa/rx@v1
        with:
          command: test
```

### MSRV verification

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: compat
    rust-toolchain: stable
```

### Release build

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: build --release
    cache: true
```

### With remote cache

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: build --release
  env:
    AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
    AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
```

### Subdirectory project

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: ci
    working-directory: backend/
```

## How it works

The action performs these steps:

1. **Install Rust toolchain** -- uses `dtolnay/rust-toolchain` with the specified toolchain and adds `clippy` and `rustfmt` components
2. **Cache artifacts** -- uses `actions/cache@v4` to cache `~/.cargo/registry`, `~/.cargo/git`, `~/.rx/cache`, and `target/`
3. **Install rx** -- downloads the prebuilt binary for the runner's platform, or falls back to `cargo install` from source
4. **Run command** -- executes `rx [--profile <profile>] <command>` in the specified working directory

## Cache key

The cache key is based on the runner OS and `Cargo.lock` hash:

```
rx-<os>-<hash of Cargo.lock>
```

A restore key of `rx-<os>-` ensures partial cache hits when dependencies change.
