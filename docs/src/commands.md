# Commands

rx provides 50+ commands organized into categories. All commands support the global flags `--quiet` (`-q`), `--verbose` (`-v`), and `--profile <name>`.

## Project setup

| Command | Description |
|---------|-------------|
| `rx init` | Generate `rx.toml` with smart defaults |
| `rx init --migrate` | Auto-detect project settings from existing tools |
| `rx init --ci` | Also generate `.github/workflows/ci.yml` |
| `rx new <name>` | Create a new Rust project |
| `rx new <name> --template <t>` | Create from template: `axum`, `cli`, `wasm`, `lib` |
| `rx config` | Show resolved configuration |

## Build and run

| Command | Description |
|---------|-------------|
| `rx build` | Build with fast linker and caching |
| `rx build --release` | Release build |
| `rx build --target <triple>` | Cross-compile for a target triple |
| `rx run [-- args...]` | Build and run the binary |
| `rx check` | Type-check without codegen (fast feedback) |
| `rx clean` | Clean build artifacts |
| `rx clean --gc` | Clean local target/ and GC global cache |
| `rx clean --all` | Clean all workspace members |

## Testing

| Command | Description |
|---------|-------------|
| `rx test` | Run tests (nextest if available) |
| `rx test --affected` | Only test packages changed since base ref |
| `rx test --affected --base main` | Changed since a specific branch |
| `rx test-smart` | Smart ordering with failure history |
| `rx test-smart --shards 4` | Distribute across parallel shards |
| `rx coverage` | Generate code coverage report |
| `rx coverage --lcov` | LCOV output for CI |
| `rx coverage --open` | Open HTML report in browser |
| `rx bench` | Run benchmarks |
| `rx test-advanced snapshot` | Snapshot testing |
| `rx test-advanced fuzz` | Fuzz testing |
| `rx test-advanced mutate` | Mutation testing |

## Code quality

| Command | Description |
|---------|-------------|
| `rx lint` | Lint with clippy (strict defaults) |
| `rx fmt` | Format code with rustfmt |
| `rx fix` | Auto-fix everything (compiler + clippy + fmt) |
| `rx ci` | Run full pipeline: fmt, clippy, test, build |

## Dependencies

| Command | Description |
|---------|-------------|
| `rx pkg add <crate>` | Add a dependency |
| `rx pkg remove <crate>` | Remove a dependency |
| `rx pkg upgrade` | Upgrade dependencies |
| `rx pkg list` | List dependencies |
| `rx pkg why <crate>` | Explain why a crate is included |
| `rx pkg dedupe` | Deduplicate dependency versions |
| `rx pkg compat` | Check dependency MSRV compatibility |
| `rx tree` | Show dependency tree |
| `rx deps` | Dependency health dashboard |
| `rx outdated` | Check for outdated dependencies |
| `rx audit` | Audit for security vulnerabilities |

## Workspace

| Command | Description |
|---------|-------------|
| `rx ws list` | List all workspace members |
| `rx ws graph` | Show dependency graph |
| `rx ws run <cmd>` | Run a command across all members |
| `rx ws script <name>` | Run a named script in each member |
| `rx ws exec "<cmd>"` | Run a shell command in each member directory |

## Cache

| Command | Description |
|---------|-------------|
| `rx cache status` | Show cache size and artifact count |
| `rx cache gc` | Remove artifacts older than 30 days |
| `rx cache purge` | Delete the entire cache |

## Toolchain

| Command | Description |
|---------|-------------|
| `rx toolchain install <ver>` | Install a Rust toolchain |
| `rx toolchain use <ver>` | Set the active toolchain |
| `rx toolchain list` | List installed toolchains |
| `rx toolchain update` | Update toolchains |

## Analysis and publishing

| Command | Description |
|---------|-------------|
| `rx doc` | Build documentation |
| `rx size` | Show binary size |
| `rx bloat` | Analyze binary bloat by function or crate |
| `rx expand` | Expand macros (requires cargo-expand) |
| `rx sbom` | Generate Software Bill of Materials |
| `rx publish` | Publish crate(s) to crates.io |
| `rx release <ver>` | Bump version, commit, tag, push |

## Environment and utilities

| Command | Description |
|---------|-------------|
| `rx doctor` | Check your development environment |
| `rx upgrade` | Update toolchains and dependencies |
| `rx self-update` | Update rx itself |
| `rx explain <code>` | Explain a Rust error code |
| `rx script <name>` | Run a script from rx.toml |
| `rx watch` | Watch for changes and rebuild |
| `rx env show` | Show resolved environment variables |
| `rx env shell` | Print export statements |
| `rx compat` | Check MSRV compatibility |
| `rx sandbox` | Sandboxed build for dependency detection |
| `rx completions <shell>` | Generate shell completions |
| `rx manpage` | Generate man page |
| `rx stats show` | View build time statistics |
| `rx stats clear` | Clear statistics |

## Background services

| Command | Description |
|---------|-------------|
| `rx daemon start` | Start background daemon |
| `rx daemon stop` | Stop daemon |
| `rx daemon status` | Check daemon status |
| `rx daemon ping` | Verify daemon responds |
| `rx worker warm` | Pre-start check/fmt/lint workers |
| `rx worker status` | Show active workers |
| `rx worker stop` | Kill all workers |

## Registries and lockfile

| Command | Description |
|---------|-------------|
| `rx registry login` | Authenticate with a registry |
| `rx registry list` | Show configured registries |
| `rx registry add <name> <url>` | Add a private registry |
| `rx lockfile check` | Check lockfile health |
| `rx lockfile enforce` | CI: fail if Cargo.lock is out of sync |

## Telemetry and plugins

| Command | Description |
|---------|-------------|
| `rx telemetry on/off/status` | Manage anonymous telemetry |
| `rx plugin list` | List available plugins |
| `rx plugin run <name>` | Run a plugin |
