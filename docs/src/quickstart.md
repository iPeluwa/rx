# Quick Start

## Create a project

```sh
rx new myproject               # binary project
rx new myapi --template axum   # Axum web API
rx new mycli --template cli    # Clap CLI
rx new mywasm --template wasm  # WASM library
```

## Daily workflow

```sh
rx build                       # build with fast linker + caching
rx run                         # build and run
rx test                        # run tests (nextest if available)
rx lint                        # clippy with strict defaults
rx fmt                         # rustfmt
rx fix                         # auto-fix everything in one pass
rx ci                          # run full CI pipeline locally
rx watch                       # rebuild on file changes
```

## Configuration

```sh
rx init                        # generate rx.toml with smart defaults
rx init --migrate              # detect existing tools and configure
rx config                      # show resolved configuration
```
