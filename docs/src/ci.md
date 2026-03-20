# CI Integration

rx is designed to work seamlessly in continuous integration environments. This section covers the official GitHub Action, VS Code extension, and general CI best practices.

## One-command CI

The simplest way to run your full CI pipeline is:

```sh
rx ci
```

This executes, in order:

1. `rx fmt --check` -- verify formatting
2. `rx lint` -- run clippy with configured severity
3. `rx test` -- run the test suite
4. `rx build` -- verify the build succeeds

## CI profiles

Use a profile to customize behavior for CI:

```toml
[profile.ci]
build = { cache = false, jobs = 2 }
lint = { severity = "deny" }
test = { runner = "nextest" }
env = { CI = "true", RUST_BACKTRACE = "1" }
```

```sh
rx --profile ci ci
```

## Lockfile enforcement

Ensure dependency versions are reproducible:

```sh
rx lockfile enforce
```

This fails the build if `Cargo.lock` is out of sync with `Cargo.toml`. See [Lockfile Policy](./advanced/lockfile.md).

## Affected-only testing

In large workspaces, only test what changed:

```sh
rx test --affected --base main
```

## MSRV verification

Verify compatibility with your declared minimum Rust version:

```sh
rx compat
```

## Recommended CI pipeline

A comprehensive CI pipeline might look like:

```sh
rx lockfile enforce
rx --profile ci ci
rx compat
rx audit
```

Or as separate steps for better CI reporting:

```yaml
steps:
  - run: rx lockfile enforce
  - run: rx fmt --check
  - run: rx lint
  - run: rx test --affected --base main
  - run: rx build --release
  - run: rx compat
  - run: rx audit
```

## Integrations

- [GitHub Action](./ci/github-action.md) -- official action for GitHub Actions
- [VS Code Extension](./ci/vscode.md) -- editor integration for local development
