# Quick Start

## Set up a project

```sh
cd my-rust-project
rx init                        # generate rx.toml with smart defaults
rx init --ci                   # also generate .github/workflows/ci.yml
rx init --migrate              # detect existing tools and configure
rx config                      # show resolved configuration
```

## Daily workflow

```sh
rx check                       # fast type-check feedback
rx build                       # build with fast linker
rx run                         # build and run
rx test                        # run tests (nextest if available)
rx lint                        # clippy with strict defaults
rx fmt                         # rustfmt
rx fix                         # auto-fix everything in one pass
rx ci                          # run full CI pipeline locally
```

## Workspaces

```sh
rx graph                       # see the dependency graph
rx ws run build                # build all members in dependency order
rx test --affected             # only test packages your change touched
```

## Tasks

Define project tasks once in `rx.toml`:

```toml
[tasks]
bench = "cargo bench"

[tasks.ci]
depends-on = ["fmt", "lint", "test", "build"]
```

Then run them locally or in CI:

```sh
rx run ci          # runs fmt, lint, test, build through the task graph
rx run bench
rx run             # list all available tasks
```

`fmt`, `lint`, `test`, `build`, and `check` are built-in tasks, so a `ci` pipeline needs no shell commands at all. Independent tasks run concurrently.
