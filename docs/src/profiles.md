# Profiles

Profiles let you override rx configuration for different contexts -- CI, development, release builds, or any custom scenario.

## Defining a profile

Add a `[profile.<name>]` section to your `rx.toml`. Each profile can override `build`, `lint`, `test`, and `env` settings:

```toml
[profile.ci]
build = { cache = false, jobs = 2 }
lint = { severity = "deny" }
test = { runner = "nextest" }
env = { CI = "true" }
```

## Using a profile

Pass `--profile <name>` to any rx command:

```sh
rx --profile ci build
rx --profile ci test
rx --profile ci lint
```

The profile flag is a global flag, so it must appear before the subcommand.

## How merging works

When a profile is active, rx starts with the base configuration (global + project) and then applies the profile overrides on top. Only the fields explicitly set in the profile are changed; everything else keeps its base value.

For example, given this configuration:

```toml
[build]
linker = "mold"
cache = true
jobs = 0

[profile.ci]
build = { cache = false, jobs = 2 }
```

Running `rx --profile ci build` uses:

| Field | Value | Source |
|-------|-------|--------|
| `linker` | `"mold"` | base config |
| `cache` | `false` | profile override |
| `jobs` | `2` | profile override |

## Common profiles

### CI profile

Disable caching (CI runners are ephemeral) and enforce strict linting:

```toml
[profile.ci]
build = { cache = false, jobs = 2 }
lint = { severity = "deny" }
test = { runner = "nextest" }
env = { CI = "true", RUST_BACKTRACE = "1" }
```

### Release profile

Use the system linker for maximum compatibility:

```toml
[profile.release]
build = { cache = true }
env = { RUSTFLAGS = "-C target-cpu=native" }
```

### Minimal profile

Fast iteration during development:

```toml
[profile.quick]
build = { cache = true }
lint = { severity = "warn" }
```

## Profile environment variables

The `env` field in a profile merges with (and overrides) the base `[env]` section:

```toml
[env]
RUST_BACKTRACE = "1"
RUST_LOG = "info"

[profile.ci]
env = { RUST_LOG = "warn", CI = "true" }
```

With `--profile ci`, `RUST_BACKTRACE` remains `"1"`, `RUST_LOG` becomes `"warn"`, and `CI` is set to `"true"`.

## Overridable fields

| Section | Fields |
|---------|--------|
| `build` | `cache`, `jobs` |
| `lint` | `severity` |
| `test` | `runner` |
| `env` | Any key-value pair |
