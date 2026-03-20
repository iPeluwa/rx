# Telemetry

rx includes an opt-in, local-only telemetry system for tracking usage patterns and build performance. Telemetry is **off by default** and data is **never sent** to any remote server.

## Commands

```sh
rx telemetry on                  # opt in to telemetry
rx telemetry off                 # opt out (default)
rx telemetry status              # show collected data
```

## What is collected

When telemetry is enabled, rx records:

- **Commands run** -- which rx commands are invoked (e.g., "build", "test", "lint")
- **Timing data** -- how long each command takes to complete
- **Cache hit/miss rates** -- whether the artifact cache was used
- **Build outcomes** -- success or failure (not error details)
- **Platform info** -- OS and architecture

## What is NOT collected

- Source code or file contents
- Error messages or stack traces
- File paths or project names
- Environment variables or secrets
- Network requests or dependency names

## Where data is stored

All telemetry data is stored locally at:

```
~/.rx/telemetry.json
```

This is a plain JSON file that you can inspect, edit, or delete at any time.

## Viewing telemetry

```sh
rx telemetry status
```

Shows a summary of collected data:

```
Telemetry: enabled
Data file: ~/.rx/telemetry.json
Records: 142

Command frequency:
  build    58 times
  test     45 times
  check    22 times
  lint     12 times
  fmt       5 times

Average build time: 12.3s
Cache hit rate: 73%
```

## Opting out

```sh
rx telemetry off
```

This stops collecting new data. Existing data remains in `~/.rx/telemetry.json` until you delete it:

```sh
rx telemetry off
rm ~/.rx/telemetry.json
```

## Privacy

- Telemetry is **off by default** -- you must explicitly opt in
- Data is **stored locally only** -- nothing is sent over the network
- Data is **human-readable JSON** -- you can inspect exactly what is recorded
- You can **delete data at any time** by removing the file
