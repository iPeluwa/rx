# rx.toml Reference

## `[build]`

| Key | Type | Default | Description |
|---|---|---|---|
| `linker` | string | `"auto"` | Linker to use: `"auto"`, `"mold"`, `"lld"`, or `"system"` |
| `rustflags` | string[] | `[]` | Extra RUSTFLAGS to append |
| `cache` | bool | `true` | Enable the global artifact cache |
| `jobs` | u32 | `0` | Parallel jobs (0 = auto-detect CPU count) |
| `incremental_link` | bool | `true` | Enable incremental linking (split-debuginfo, --as-needed) |
| `remote_cache` | string | `""` | Remote cache URL: `"s3://bucket/prefix"`, `"gs://bucket/prefix"`, or `"/path"` |

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

## `[watch]`

| Key | Type | Default | Description |
|---|---|---|---|
| `cmd` | string | `"build"` | Default command on file changes |
| `ignore` | string[] | `[]` | File patterns to ignore (e.g. `["*.log", "tmp/**"]`) |

## `[scripts]`

Key-value pairs of script names to shell commands:

```toml
[scripts]
ci = "cargo fmt --check && cargo clippy -- -D warnings && cargo test"
deploy = "cargo build --release && scp target/release/myapp server:/opt/"
```

## `[env]`

Key-value pairs of environment variables set for all rx commands:

```toml
[env]
RUST_BACKTRACE = "1"
DATABASE_URL = "postgres://localhost/dev"
```

## `[profile.<name>]`

Override settings per context. See [Profiles](./profiles.md).
