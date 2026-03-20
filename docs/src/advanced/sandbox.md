# Build Sandbox

The `rx sandbox` command runs your build in an isolated environment to detect undeclared dependencies on system tools, environment variables, or global state.

## Usage

```sh
rx sandbox                       # sandboxed debug build
rx sandbox --release             # sandboxed release build
```

## How it works

`rx sandbox` runs `cargo build` with a stripped environment:

1. **Environment cleared** -- all environment variables are removed via `env_clear()`
2. **Essential vars restored** -- only the minimum required variables are set:
   - `HOME`
   - `CARGO_HOME`
   - `RUSTUP_HOME`
   - `PATH` (minimal, only essential directories)
3. **Build executed** -- cargo runs in this minimal environment

## What it detects

### Undeclared environment dependencies

Build scripts (`build.rs`) or procedural macros that rely on environment variables not explicitly set in `Cargo.toml`:

```
# Fails in sandbox because $MY_SECRET_KEY is not available
error: environment variable `MY_SECRET_KEY` not set
```

### System tool dependencies

Code or build scripts that shell out to tools not in the minimal PATH:

```
# Fails in sandbox because `protoc` is not on the minimal PATH
error: failed to execute `protoc`: No such file or directory
```

### Global state dependencies

Any dependency on globally-installed files, caches, or configurations that would not be present on a clean machine.

## When to use it

- **Before publishing** -- ensure your crate builds on a clean machine
- **In CI** -- catch environment-specific assumptions early
- **After adding build scripts** -- verify that `build.rs` dependencies are properly declared
- **Debugging "works on my machine" issues** -- isolate what external state the build depends on

## Fixing sandbox failures

When `rx sandbox` fails, the error message indicates what the build depends on:

| Failure type | Fix |
|-------------|-----|
| Missing env var | Declare it in `Cargo.toml` `[env]` or set it in `build.rs` |
| Missing tool | Add it to your documented build requirements |
| Missing file | Use `include_str!` or `include_bytes!` for embedded resources |

## Limitations

- The sandbox does not use containerization (no Docker or chroot). It only strips the environment.
- System libraries linked via `-l` flags are still available from the system.
- The sandbox is focused on environment isolation, not filesystem isolation.
