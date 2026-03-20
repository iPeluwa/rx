# Private Registries

rx supports configuring and authenticating with private crate registries for teams that host internal crates.

## Commands

```sh
rx registry list                 # show configured registries
rx registry add <name> <url>     # add a new registry
rx registry login <name>         # interactive authentication
rx registry login <name> <token> # token-based authentication
```

## Adding a registry

```sh
rx registry add my-registry https://index.example.com
```

This registers the registry name and index URL. The registry can then be used in `Cargo.toml`:

```toml
[dependencies]
my-internal-crate = { version = "1.0", registry = "my-registry" }
```

## Authentication

### Interactive login

```sh
rx registry login my-registry
```

Prompts for credentials interactively. The token is stored securely in your credential store.

### Token-based login

```sh
rx registry login my-registry tok-abc123
```

Stores the provided token directly. Useful for CI environments where interactive prompts are not available.

## Listing registries

```sh
rx registry list
```

Shows all configured registries with their names and index URLs:

```
Registries:
  crates-io       https://github.com/rust-lang/crates.io-index (default)
  my-registry     https://index.example.com
  team-registry   https://internal.company.com/index
```

## CI usage

In CI, pass the registry token as an environment variable:

```yaml
- uses: iPeluwa/rx@v1
  with:
    command: build
  env:
    CARGO_REGISTRIES_MY_REGISTRY_TOKEN: ${{ secrets.REGISTRY_TOKEN }}
```

Cargo reads registry tokens from `CARGO_REGISTRIES_<NAME>_TOKEN` environment variables, where `<NAME>` is the uppercase registry name with hyphens replaced by underscores.

## How it works

rx manages registry configuration by writing to Cargo's native config files:

- `~/.cargo/config.toml` -- registry index URLs
- `~/.cargo/credentials.toml` -- authentication tokens

This means registries configured via rx work with plain `cargo` commands as well.

## Troubleshooting

If authentication fails:

1. Verify the registry URL: `rx registry list`
2. Re-authenticate: `rx registry login <name>`
3. Check that the token has not expired
4. For CI, verify the environment variable name matches the registry name
