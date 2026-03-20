# MSRV Compatibility

rx can verify that your code and dependencies are compatible with your declared Minimum Supported Rust Version (MSRV).

## Usage

```sh
rx compat                        # check code + deps against rust-version
rx pkg compat                    # same, via the pkg subcommand
```

## How it works

1. **Read MSRV** -- rx reads the `rust-version` field from your `Cargo.toml`:

    ```toml
    [package]
    rust-version = "1.70"
    ```

2. **Install toolchain** -- if the MSRV toolchain is not already installed, rx installs it automatically via `rustup toolchain install`.

3. **Check code** -- rx runs `cargo check --all-targets` using the MSRV toolchain to verify that your code compiles on the minimum version.

4. **Check dependencies** -- rx inspects dependency metadata (via `cargo metadata`) to verify that each dependency's own `rust-version` (if declared) is compatible with your MSRV.

## What it catches

- Code that uses language features or standard library APIs introduced after your declared MSRV
- Dependencies that require a newer Rust version than your MSRV
- Conditional compilation (`#[cfg]`) issues that only appear on older compilers

## Example output

```
Checking MSRV compatibility (rust-version = 1.70)
Installing toolchain: 1.70.0
Running: cargo +1.70.0 check --all-targets

error[E0658]: use of unstable library feature 'is_some_and'
  --> src/main.rs:42:10

Dependency MSRV check:
  serde 1.0.190     rust-version: 1.56  OK
  tokio 1.34.0      rust-version: 1.70  OK
  axum 0.7.1        rust-version: 1.75  INCOMPATIBLE (requires 1.75, you declared 1.70)
```

## CI usage

Add MSRV checking to your CI pipeline:

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: compat
```

Or include it in your `rx.toml` scripts:

```toml
[scripts]
ci = "rx fmt --check && rx lint && rx test && rx compat"
```

## Setting the MSRV

Add or update the `rust-version` field in your `Cargo.toml`:

```toml
[package]
name = "my-crate"
version = "0.1.0"
rust-version = "1.70"
```

If no `rust-version` is declared, `rx compat` reports an error and exits.

## Tips

- Run `rx compat` before publishing to crates.io to ensure your declared MSRV is accurate
- When adding new dependencies, check their `rust-version` to avoid accidentally raising your MSRV
- Use `rx pkg compat` to focus specifically on dependency compatibility
