# Dependencies

rx provides a comprehensive set of dependency management commands under `rx pkg`, plus standalone commands for analysis and auditing.

## Adding and removing dependencies

```sh
rx pkg add serde                 # add serde to [dependencies]
rx pkg add tokio --features full # add with features
rx pkg add --dev insta           # add to [dev-dependencies]
rx pkg remove serde              # remove a dependency
```

## Upgrading dependencies

```sh
rx pkg upgrade                   # upgrade all dependencies
rx pkg upgrade serde             # upgrade a specific dependency
```

## Listing dependencies

```sh
rx pkg list                      # list direct dependencies
```

## Dependency tree

```sh
rx tree                          # show full dependency tree
```

Displays the complete dependency graph in a tree format, showing transitive dependencies and their versions.

## Dependency health dashboard

```sh
rx deps
```

`rx deps` provides a combined view of dependency health by running:

1. Dependency tree analysis
2. Outdated dependency check
3. Security audit

This gives a single-command overview of your project's dependency status.

## Checking for outdated dependencies

```sh
rx outdated                      # list dependencies with newer versions
```

Shows which dependencies have newer versions available on crates.io.

## Security audit

```sh
rx audit                         # audit dependencies for known vulnerabilities
```

Checks your dependency tree against the RustSec advisory database for known security vulnerabilities.

## Explaining dependencies

```sh
rx pkg why serde_json            # explain why this crate is in your tree
```

Shows the dependency chain(s) that bring a crate into your project. Useful for understanding why a transitive dependency exists.

## Deduplication

```sh
rx pkg dedupe                    # deduplicate dependency versions
```

Identifies cases where multiple versions of the same crate exist in your dependency tree and attempts to consolidate them.

## MSRV compatibility

```sh
rx pkg compat                    # check dependency MSRV compatibility
rx compat                        # same, top-level alias
```

Verifies that all dependencies are compatible with your declared `rust-version` in `Cargo.toml`. See [MSRV Compatibility](../advanced/msrv.md) for details.

## Workflow example

A typical dependency maintenance workflow:

```sh
# Check what's outdated
rx outdated

# Upgrade everything
rx pkg upgrade

# Make sure nothing broke
rx test

# Audit for vulnerabilities
rx audit

# Or do it all at once
rx deps
```
