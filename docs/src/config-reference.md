# rx.toml Reference

## `[build]`

| Key | Type | Default | Description |
|---|---|---|---|
| `linker` | string | `"auto"` | Linker to use: `"auto"`, `"mold"`, `"lld"`, or `"system"` |
| `rustflags` | string[] | `[]` | Extra RUSTFLAGS to append |
| `cache` | bool | `false` | Enable the global artifact cache (opt-in; fingerprint does not yet cover target triple, features, toolchain, or build scripts — see PRODUCT.md) |
| `jobs` | u32 | `0` | Parallel jobs (0 = auto-detect CPU count) |
| `incremental_link` | bool | `true` | Enable incremental linking (split-debuginfo, --as-needed) |

## `[test]`

| Key | Type | Default | Description |
|---|---|---|---|
| `runner` | string | `"auto"` | Test runner: `"auto"`, `"nextest"`, or `"cargo"` |
| `extra_args` | string[] | `[]` | Extra arguments always passed to the test runner |

## `[lint]`

| Key | Type | Default | Description |
|---|---|---|---|
| `severity` | string | `"deny"` | Clippy severity: `"deny"`, `"warn"`, or `"allow"` |
| `extra_lints` | string[] | `[]` | Extra clippy lints (e.g. `"clippy::pedantic"`) |

## `[fmt]`

| Key | Type | Default | Description |
|---|---|---|---|
| `extra_args` | string[] | `[]` | Extra rustfmt arguments |

## `[tasks]`

Named tasks for `rx run`. A task is a shell command (string shorthand), or a table with a `command` and/or `depends-on` list. Built-in tasks (`fmt`, `lint`, `test`, `build`, `check`, `ci`) are used for names you don't define; defining the name overrides the built-in.

```toml
[tasks]
deploy = "cargo build --release && scp target/release/myapp server:/opt/"

[tasks.ci]
depends-on = ["fmt", "lint", "test", "build"]
```

## `[scripts]` (deprecated)

Legacy alias for `[tasks]`: entries are plain `name = "command"` pairs and are merged into the task table (`[tasks]` wins on name collisions).

## `[env]`

Key-value pairs of environment variables set for all rx commands:

```toml
[env]
RUST_BACKTRACE = "1"
DATABASE_URL = "postgres://localhost/dev"
```

## `[profile.<name>]`

Override settings per context. See [Profiles](./profiles.md).
