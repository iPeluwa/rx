use anyhow::{Context, Result};
use std::fs;

pub fn generate_ci(provider: &str) -> Result<()> {
    match provider {
        "github" => generate_github_actions(),
        "gitlab" => generate_gitlab_ci(),
        "circle" => generate_circle_ci(),
        other => {
            anyhow::bail!(
                "unknown CI provider: {other}\n\
                 supported: github, gitlab, circle"
            );
        }
    }
}

fn generate_github_actions() -> Result<()> {
    let dir = ".github/workflows";
    fs::create_dir_all(dir).context("failed to create .github/workflows")?;

    let workflow = r#"name: CI

on:
  push:
    branches: [master, main]
  pull_request:

env:
  CARGO_TERM_COLOR: always
  RUSTFLAGS: "-D warnings"

jobs:
  check:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
        with:
          components: clippy, rustfmt
      - uses: Swatinem/rust-cache@v2
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release

  test-macos:
    runs-on: macos-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - uses: Swatinem/rust-cache@v2
      - run: cargo test

  msrv:
    runs-on: ubuntu-latest
    steps:
      - uses: actions/checkout@v4
      - uses: dtolnay/rust-toolchain@stable
      - name: Read MSRV
        id: msrv
        run: echo "version=$(grep rust-version Cargo.toml | head -1 | sed 's/.*= *\"\(.*\)\"/\1/')" >> "$GITHUB_OUTPUT"
      - uses: dtolnay/rust-toolchain@master
        with:
          toolchain: ${{ steps.msrv.outputs.version }}
      - run: cargo check
"#;

    fs::write(format!("{dir}/ci.yml"), workflow)?;
    crate::output::success("created .github/workflows/ci.yml");
    Ok(())
}

fn generate_gitlab_ci() -> Result<()> {
    let config = r#"image: rust:latest

stages:
  - check
  - test
  - build

variables:
  CARGO_HOME: ${CI_PROJECT_DIR}/.cargo

cache:
  key: ${CI_COMMIT_REF_SLUG}
  paths:
    - .cargo/registry
    - target/

fmt:
  stage: check
  script:
    - rustup component add rustfmt
    - cargo fmt --check

clippy:
  stage: check
  script:
    - rustup component add clippy
    - cargo clippy -- -D warnings

test:
  stage: test
  script:
    - cargo test

build:
  stage: build
  script:
    - cargo build --release
  artifacts:
    paths:
      - target/release/
"#;

    fs::write(".gitlab-ci.yml", config)?;
    crate::output::success("created .gitlab-ci.yml");
    Ok(())
}

fn generate_circle_ci() -> Result<()> {
    let dir = ".circleci";
    fs::create_dir_all(dir).context("failed to create .circleci")?;

    let config = r#"version: 2.1

executors:
  rust:
    docker:
      - image: cimg/rust:1.82

jobs:
  check:
    executor: rust
    steps:
      - checkout
      - run: rustup component add clippy rustfmt
      - run: cargo fmt --check
      - run: cargo clippy -- -D warnings
      - run: cargo test
      - run: cargo build --release

workflows:
  ci:
    jobs:
      - check
"#;

    fs::write(format!("{dir}/config.yml"), config)?;
    crate::output::success("created .circleci/config.yml");
    Ok(())
}
