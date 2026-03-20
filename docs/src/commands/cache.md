# Cache

rx maintains a global content-addressed artifact cache at `~/.rx/cache`. The cache stores compiled artifacts keyed by a fingerprint of your source code, configuration, and build profile.

## Commands

```sh
rx cache status                  # show cache size and artifact count
rx cache gc                      # remove artifacts older than 30 days
rx cache purge                   # delete the entire cache
```

## How the cache works

Every `rx build` follows this flow:

### 1. Mtime fast-path

rx records the modification time of every source file after each build. On the next build, it checks whether any file has a newer mtime. If nothing changed, the cached fingerprint is reused instantly without reading any file contents.

### 2. Content fingerprinting

If an mtime mismatch is detected, rx computes a full **xxHash (xxh3-128)** fingerprint from:

- `Cargo.toml` contents
- `Cargo.lock` contents
- All source files
- Build profile (debug or release)
- Active RUSTFLAGS

### 3. Cache hit

If a cached build matches the fingerprint, artifacts are restored into `target/` using the fastest available method:

1. **Reflink** (copy-on-write) -- instant on APFS and btrfs
2. **Hardlink** -- instant, shares the inode
3. **Regular copy** -- fallback, always works

The build is skipped entirely.

### 4. Cache miss

The build runs normally via cargo. After completion, artifacts are stored in the cache for future use.

## Integrity guarantees

The cache is designed for correctness under concurrent access:

- **Atomic writes** -- the cache index and mtime snapshots are written to a temp file and atomically renamed
- **File locking** -- a lock file at `~/.rx/cache/lock` prevents concurrent rx processes from racing on the index
- **Staging directory** -- new artifacts are written to a staging directory, then renamed into place once complete
- **Parallel I/O** -- `rayon` is used for parallel file copies during store and restore

## Cache layout

```
~/.rx/cache/
├── index.json              # fingerprint -> artifact metadata
├── mtime-snapshot.json     # per-file modification times
├── lock                    # file lock for concurrent access
└── artifacts/
    └── <fingerprint>/      # one directory per unique build
        ├── deps/
        ├── build/
        └── ...
```

## Garbage collection

```sh
rx cache gc                      # remove entries older than 30 days
```

Garbage collection removes cache entries that have not been accessed within 30 days. The cache index is updated atomically.

## Cleaning

```sh
rx clean                         # clean local target/ directory
rx clean --gc                    # clean target/ and GC global cache
rx clean --all                   # clean all workspace member target/ dirs
rx cache purge                   # delete the entire global cache
```

## Remote cache

rx can push and pull cache artifacts to a remote backend for sharing across CI runners and team members. See [Remote Cache](../advanced/remote-cache.md) for configuration details.

## Disabling the cache

Disable caching globally or per-profile:

```toml
[build]
cache = false

# or per-profile:
[profile.ci]
build = { cache = false }
```
