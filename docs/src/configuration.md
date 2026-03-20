# Configuration

rx uses a TOML configuration file called `rx.toml` to control build, test, lint, format, and watch behavior. Configuration is optional -- rx works out of the box with sensible defaults.

## Generating rx.toml

```sh
rx init              # generate rx.toml with smart defaults
rx init --migrate    # detect existing tools and generate tailored config
rx init --ci         # also generate .github/workflows/ci.yml
```

`rx init` inspects your project and applies smart defaults:

- Workspaces get a `ci` script automatically
- If `mold` or `lld` is detected, it is set as the default linker
- If `cargo-nextest` is installed, the test runner is set to `"auto"`

`rx init --migrate` goes further by detecting Makefiles, benchmarks, error handling crates, and other project-specific patterns.

## File structure

```toml
[build]
linker = "auto"            # "auto", "mold", "lld", or "system"
rustflags = []             # extra RUSTFLAGS
cache = true               # enable global artifact cache
jobs = 0                   # parallel jobs (0 = auto-detect CPU count)
incremental_link = true    # split-debuginfo and --as-needed
remote_cache = ""          # "s3://bucket/prefix", "gs://bucket/prefix", or "/path"

[test]
runner = "auto"            # "auto", "nextest", or "cargo"
extra_args = []

[lint]
severity = "deny"          # "deny", "warn", or "allow"
extra_lints = []           # e.g. ["clippy::pedantic"]

[fmt]
extra_args = []

[watch]
cmd = "build"              # default command on file changes
ignore = []                # patterns to ignore, e.g. ["*.log", "tmp/**"]

[scripts]
ci = "cargo fmt --check && cargo clippy -- -D warnings && cargo test"
bench = "cargo bench"

[env]
RUST_BACKTRACE = "1"
```

## Global vs project config

rx resolves configuration by merging two files:

1. **Global config** at `~/.rx/config.toml` -- applies to all projects
2. **Project config** at `./rx.toml` -- applies to the current project

Project values override global values. This lets you set personal defaults (like a preferred linker) globally while allowing each project to customize as needed.

```sh
rx config    # show the fully resolved configuration
```

## Unknown key warnings

rx validates every key in `rx.toml`. If you mistype a key name, rx prints a warning rather than silently ignoring it:

```
warning: unknown key `buld` in rx.toml (did you mean `build`?)
```

This catches typos that would otherwise lead to confusing behavior.

## Environment variables

The `[env]` section sets environment variables for all rx commands:

```toml
[env]
RUST_BACKTRACE = "1"
RUST_LOG = "info"
```

You can also manage env vars with the `rx env` command:

```sh
rx env show     # display resolved environment variables
rx env shell    # print export statements for your shell
```

## Profiles

Override any configuration per context with `[profile.<name>]`. See the [Profiles](./profiles.md) chapter for details.

## See also

- [rx.toml Reference](./config-reference.md) -- full field reference
- [Profiles](./profiles.md) -- profile-based overrides
