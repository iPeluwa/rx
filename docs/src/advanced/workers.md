# Persistent Workers

rx supports persistent background workers that keep check, format, and lint processes warm. This eliminates cold-start overhead for frequently-run commands.

## Commands

```sh
rx worker warm                   # start check, fmt, and lint workers
rx worker status                 # show active worker processes
rx worker stop                   # kill all workers
```

## How it works

When you run `rx worker warm`, rx starts persistent background processes for:

- **check** -- a warm `cargo check` process ready to type-check on demand
- **fmt** -- a warm `rustfmt` process ready to format on demand
- **lint** -- a warm `clippy` process ready to lint on demand

These workers stay alive between invocations. When you run `rx check`, `rx fmt`, or `rx lint`, the command connects to the existing worker process instead of spawning a new one from scratch.

## Benefits

Worker processes maintain:

- Loaded and parsed crate metadata
- Compiled proc-macro libraries
- Incremental compilation state
- Warm filesystem caches

This means the first `rx check` after `rx worker warm` is fast, and subsequent checks are even faster.

## Checking worker status

```sh
rx worker status
```

Shows which workers are currently running, their PIDs, and uptime:

```
Workers:
  check  PID 12345  uptime 2h 15m
  fmt    PID 12346  uptime 2h 15m
  lint   PID 12347  uptime 2h 15m
```

## Stopping workers

```sh
rx worker stop
```

Terminates all active worker processes. Workers are also automatically stopped when the daemon shuts down (if using `rx daemon`).

## When to use workers

Workers are most useful during active development when you are:

- Running `rx check` repeatedly as you edit code
- Using `rx watch` for continuous feedback
- Working in a large codebase where compilation startup time is noticeable

For CI or infrequent builds, workers provide less benefit and can be skipped.

## Resource usage

Each worker holds an open process and some memory for cached state. On a typical project, the three workers together use 50-200 MB of memory. They do not consume CPU when idle.

Stop workers when you are done developing to reclaim resources:

```sh
rx worker stop
```
