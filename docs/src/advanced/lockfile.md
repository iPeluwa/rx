# Lockfile Policy

rx provides commands to check lockfile health and enforce lockfile consistency in CI. This prevents issues caused by out-of-sync or missing `Cargo.lock` files.

## Commands

```sh
rx lockfile check                # check lockfile health
rx lockfile enforce              # CI: fail if Cargo.lock is out of sync
```

## Lockfile check

```sh
rx lockfile check
```

Performs several health checks on your `Cargo.lock`:

1. **Existence** -- verifies that `Cargo.lock` exists
2. **Freshness** -- checks that the lockfile is up to date with `Cargo.toml`
3. **Git status** -- warns if `Cargo.lock` has uncommitted changes
4. **Format** -- validates that the lockfile is well-formed

Example output:

```
Lockfile health check:
  Exists:     OK
  Fresh:      OK
  Git clean:  WARNING (uncommitted changes)
  Format:     OK
```

## Lockfile enforce

```sh
rx lockfile enforce
```

A strict mode for CI that fails the build if `Cargo.lock` is not perfectly in sync. This catches cases where a developer updated `Cargo.toml` but forgot to commit the updated lockfile.

`rx lockfile enforce` exits with a non-zero status if:

- `Cargo.lock` does not exist
- Running `cargo generate-lockfile` would produce a different file
- The lockfile has parse errors

## CI usage

Add lockfile enforcement to your CI pipeline:

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: lockfile enforce
```

Or include it in your CI script:

```toml
[scripts]
ci = "rx lockfile enforce && rx fmt --check && rx lint && rx test"
```

## Why enforce the lockfile?

Without lockfile enforcement, CI builds may use different dependency versions than what developers tested locally. This can cause:

- **Non-reproducible builds** -- different CI runs resolve different versions
- **Subtle test failures** -- a dependency update introduces a regression
- **Security gaps** -- an unreviewed dependency update introduces a vulnerability

By committing `Cargo.lock` and enforcing it in CI, you ensure that every build uses exactly the same dependency versions.

## Best practices

- Always commit `Cargo.lock` for binary projects and applications
- For libraries, committing `Cargo.lock` is optional but recommended for CI reproducibility
- Run `rx lockfile check` locally before pushing to catch issues early
- Use `rx lockfile enforce` in CI as a gating check
