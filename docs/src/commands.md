# Commands

rx has a deliberately small command surface. All commands support the global flags `--quiet` (`-q`), `--verbose` (`-v`), and `--profile <name>`.

## Project setup

| Command | Description |
|---------|-------------|
| `rx init` | Generate `rx.toml` with smart defaults |
| `rx init --migrate` | Auto-detect project settings from existing tools |
| `rx init --ci` | Also generate `.github/workflows/ci.yml` |
| `rx config` | Show resolved configuration |

## Build and run

| Command | Description |
|---------|-------------|
| `rx build` | Build with fast linker |
| `rx build --release` | Release build |
| `rx build --target <triple>` | Cross-compile for a target triple |
| `rx run [-- args...]` | Build and run the binary |
| `rx check` | Type-check without codegen (fast feedback) |
| `rx clean` | Clean build artifacts |
| `rx clean --gc` | Clean local target/ and GC global cache |
| `rx clean --all` | Clean all workspace members |

## Testing and code quality

| Command | Description |
|---------|-------------|
| `rx test` | Run tests (nextest if available) |
| `rx test --affected` | Only test packages changed since base ref |
| `rx test --affected --base main` | Changed since a specific branch |
| `rx lint` | Lint with clippy (strict defaults) |
| `rx fmt` | Format code with rustfmt |
| `rx fix` | Auto-fix everything (compiler + clippy + fmt) |
| `rx ci` | Run full pipeline: fmt, clippy, test, build |

## Tasks

| Command | Description |
|---------|-------------|
| `rx script <name>` | Run a script defined in rx.toml |
| `rx script` | List available scripts |

## Workspace

| Command | Description |
|---------|-------------|
| `rx graph` | Show the workspace dependency graph |
| `rx ws list` | List all workspace members |
| `rx ws run <cmd>` | Run a cargo command across members in dependency order |
| `rx ws script <name>` | Run an rx.toml script across members |
| `rx ws exec <cmd>` | Run a shell command in each member directory |

## Maintenance

| Command | Description |
|---------|-------------|
| `rx cache status/gc/purge` | Manage the opt-in global artifact cache |
| `rx doctor` | Check your development environment |
| `rx stats show/clear` | View or clear build time statistics |
| `rx completions <shell>` | Generate shell completions |

## Removed commands

The 0.2 scope reset removed the toolchain-manager surface (`new`, `pkg`, `toolchain`, `release`, `publish`, `audit`, `outdated`, `tree`, `deps`, `doc`, `size`, `bloat`, `coverage`, `bench`, `expand`, `compat`, `upgrade`, `self-update`, `sbom`, `telemetry`, `plugin`, `registry`, `lockfile`, `env`, `insights`, `explain`, `manpage`, `sandbox`, `watch`, `daemon`, `worker`, `test-smart`, `test-advanced`). Use the dedicated tools directly (rustup, cargo and its plugins, sccache, cargo-release, …) or invoke them from a configured rx script. See [PRODUCT.md](https://github.com/iPeluwa/rx/blob/master/PRODUCT.md) for the reasoning.
