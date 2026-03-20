# Contributing to rx

Thanks for your interest in contributing to rx! Here's how to get started.

## Development Setup

1. **Clone the repo:**
   ```bash
   git clone https://github.com/iPeluwa/rx.git
   cd rx
   ```

2. **Build:**
   ```bash
   cargo build
   ```

3. **Run tests:**
   ```bash
   cargo test --all-targets
   ```

4. **Run clippy and fmt:**
   ```bash
   cargo clippy --all-targets -- -D warnings
   cargo fmt --check
   ```

## MSRV

The minimum supported Rust version is **1.85.0** (Rust 2024 edition). Do not use language features that require a newer compiler.

## Guidelines

- Keep PRs focused — one feature or fix per PR.
- Add tests for new functionality.
- Run `cargo clippy` and `cargo fmt` before submitting.
- Follow existing code style and conventions.

## Reporting Issues

Open an issue at https://github.com/iPeluwa/rx/issues with:
- What you expected to happen
- What actually happened
- Steps to reproduce
- Your OS and Rust version (`rustc --version`)

## License

By contributing, you agree that your contributions will be licensed under the MIT License.
