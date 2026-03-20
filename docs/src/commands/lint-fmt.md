# Lint & Format

rx wraps `clippy` and `rustfmt` into unified `rx lint` and `rx fmt` commands with configurable severity and a combined `rx fix` that applies all auto-fixes in one step.

## Linting

```sh
rx lint                          # lint with clippy
rx lint --release                # lint in release mode
```

### Severity configuration

Control how clippy warnings are treated:

```toml
[lint]
severity = "deny"     # treat all warnings as errors (default)
severity = "warn"     # show warnings but don't fail
severity = "allow"    # suppress warnings entirely
```

### Extra lints

Add additional clippy lint groups or specific lints:

```toml
[lint]
extra_lints = ["clippy::pedantic", "clippy::nursery"]
```

These are passed as additional `-W` flags to clippy.

## Formatting

```sh
rx fmt                           # format all code
rx fmt --check                   # check formatting without modifying files
```

### Extra arguments

Pass additional arguments to rustfmt:

```toml
[fmt]
extra_args = ["--edition", "2021"]
```

## Auto-fix everything

`rx fix` combines all auto-fix capabilities in a single command:

```sh
rx fix
```

This runs, in order:

1. **Compiler suggestions** -- applies `rustc` fix suggestions
2. **Clippy fixes** -- applies clippy auto-fix suggestions
3. **Formatting** -- runs rustfmt on all files

After running `rx fix`, your code should be free of all auto-fixable issues.

## CI pipeline

`rx ci` runs the full quality pipeline in order:

```sh
rx ci
```

This executes:

1. `rx fmt --check` -- verify formatting
2. `rx lint` -- run clippy
3. `rx test` -- run tests
4. `rx build` -- verify build succeeds

Use `rx ci` locally before pushing to catch issues before CI runs.

## Profile overrides

Override lint severity per profile:

```toml
[lint]
severity = "warn"           # relaxed defaults for local dev

[profile.ci]
lint = { severity = "deny" } # strict in CI
```

```sh
rx --profile ci lint         # fails on any warning
```

## Flags reference

### rx lint

| Flag | Description |
|------|-------------|
| `--release` | Lint in release mode |
| `--verbose` | Show detailed clippy output |
| `--quiet` | Suppress non-error output |

### rx fmt

| Flag | Description |
|------|-------------|
| `--check` | Check formatting without modifying files |
| `--verbose` | Show formatted file paths |
| `--quiet` | Suppress non-error output |
