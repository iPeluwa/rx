# Workspace

The `rx ws` commands provide dependency-aware orchestration for Cargo workspaces. Commands are executed across workspace members in parallel waves based on the dependency graph.

## Listing members

```sh
rx ws list
```

Lists all workspace members with their paths and versions.

## Dependency graph

```sh
rx ws graph
```

Displays the inter-package dependency graph of the workspace, showing which packages depend on which.

## Running commands

```sh
rx ws run build                  # build all members
rx ws run test                   # test all members
rx ws run test --release         # test all members in release mode
rx ws run check                  # check all members
```

`rx ws run` executes an rx command across all workspace members in dependency order. Independent packages run in parallel; dependent packages wait for their dependencies to complete.

### Parallel waves

rx uses Kahn's algorithm for topological sorting to group packages into parallel "waves":

```
Wave 1: [core, utils]           # no inter-dependencies
Wave 2: [api, cli]              # depend on core/utils
Wave 3: [integration-tests]     # depends on api/cli
```

All packages in a wave run concurrently. The next wave starts only after the current wave completes.

## Running scripts

```sh
rx ws script ci                  # run "ci" script in each member
rx ws script bench               # run "bench" script in each member
```

Runs a named script (defined in each member's `rx.toml`) across all workspace members. Members without the script are skipped.

## Running shell commands

```sh
rx ws exec "wc -l src/*.rs"      # count lines in each member
rx ws exec "cargo doc"           # run arbitrary cargo commands
rx ws exec "echo \$PWD"         # show each member's directory
```

`rx ws exec` runs an arbitrary shell command in each member's directory. Unlike `rx ws run`, this does not use dependency ordering -- commands run in parallel across all members.

## Semantic fingerprinting in workspaces

When running workspace commands, rx uses [semantic fingerprinting](../advanced/semantic-fingerprint.md) to skip rebuilding members whose public API has not changed. If `core` only had a comment edit, downstream packages like `api` and `cli` are not rebuilt.

## Example workflow

A typical CI setup for a workspace:

```sh
# Check all members in dependency order
rx ws run check

# Test only affected packages
rx test --affected --base main

# Build everything
rx ws run build --release
```

Or use the `rx ci` command which handles the full pipeline automatically.

## Cycle detection

rx detects dependency cycles in the workspace graph and reports them as errors. Cargo workspaces should not have cycles, but rx provides a clear error message if one is found.
