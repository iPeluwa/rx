# Background Daemon

rx includes a background daemon (`rxd`) that holds workspace state in memory for instant command startup. The daemon eliminates cold-start overhead by keeping configuration, dependency graphs, and fingerprint caches loaded.

## Commands

```sh
rx daemon start                  # start the daemon (foreground)
rx daemon stop                   # stop the running daemon
rx daemon status                 # check if the daemon is running
rx daemon ping                   # verify the daemon responds
```

## How it works

The daemon runs as a persistent background process and communicates via a Unix domain socket at `~/.rx/rxd.sock`.

When the daemon is running:

1. **Config is pre-loaded** -- `rx.toml` and global config are parsed once and held in memory
2. **Dependency graph is cached** -- workspace member relationships and topological ordering are computed once
3. **Fingerprint cache is warm** -- recent fingerprints are kept in memory, avoiding disk reads
4. **Commands connect via socket** -- rx CLI commands check for the daemon socket and delegate state lookups to it

## Socket protocol

The daemon listens on `~/.rx/rxd.sock` using a simple request-response protocol:

- **ping** -- health check, returns "pong"
- **config** -- returns the resolved configuration
- **fingerprint** -- returns cached fingerprint for a given path
- **workspace** -- returns workspace member list and dependency graph
- **invalidate** -- clears cached state (called when files change)

## Lifecycle

### Starting

```sh
rx daemon start
```

The daemon starts in the foreground and logs to stdout. It loads the project configuration and workspace graph on startup.

### Automatic invalidation

When source files change, the daemon invalidates affected fingerprints. If `rx.toml` changes, the entire configuration cache is reloaded.

### Stopping

```sh
rx daemon stop
```

Sends a shutdown message to the daemon via the socket. The daemon exits cleanly.

## When to use it

The daemon is most beneficial for:

- **Large workspaces** -- where loading the dependency graph takes noticeable time
- **Rapid iteration** -- when running `rx check` or `rx build` repeatedly
- **IDE integration** -- the VS Code extension can leverage the daemon for faster feedback

For small projects, the startup overhead of rx is already minimal, and the daemon provides less benefit.

## Troubleshooting

If the daemon is unresponsive:

```sh
rx daemon status                 # check if it's running
rx daemon ping                   # test responsiveness
rx daemon stop                   # stop and restart
rx daemon start
```

The socket file is at `~/.rx/rxd.sock`. If it exists but the daemon is not running, delete it manually:

```sh
rm ~/.rx/rxd.sock
rx daemon start
```
