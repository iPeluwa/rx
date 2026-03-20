# Remote Cache

rx can share build artifacts across CI runners and developer machines by pushing and pulling from a remote cache backend. This avoids redundant builds when the same code has already been compiled elsewhere.

## Supported backends

- **S3** -- Amazon S3 or S3-compatible storage (requires `aws` CLI)
- **GCS** -- Google Cloud Storage (requires `gsutil` CLI)
- **Local path** -- a shared filesystem path (NFS, network drive, or local directory)

## Configuration

Set the `remote_cache` field in `rx.toml`:

```toml
[build]
remote_cache = "s3://my-bucket/rx-cache"
```

### S3

```toml
[build]
remote_cache = "s3://my-bucket/rx-cache"
```

Requires the `aws` CLI to be installed and configured with appropriate credentials. rx runs `aws s3 cp` under the hood.

Authentication is handled by the standard AWS credential chain (environment variables, `~/.aws/credentials`, IAM roles, etc.).

### GCS

```toml
[build]
remote_cache = "gs://my-bucket/rx-cache"
```

Requires `gsutil` to be installed and authenticated. rx runs `gsutil cp` under the hood.

### Local/network path

```toml
[build]
remote_cache = "/shared/build-cache"
```

Uses a local filesystem path. This works with NFS mounts, shared network drives, or any directory accessible to all machines.

## How it works

1. After a local cache miss, rx checks the remote cache for a matching fingerprint
2. If found, the compressed tarball is downloaded and extracted into the local cache
3. If not found, the build runs normally
4. After a successful build, artifacts are compressed and uploaded to the remote cache

Artifacts are keyed by their xxHash fingerprint and stored as compressed tarballs. The remote key includes a two-character prefix for sharding:

```
<prefix>/ab/abcdef1234567890.tar.gz
```

## CI usage

A typical CI setup:

```toml
# rx.toml
[build]
remote_cache = "s3://ci-cache/rx"

[profile.ci]
build = { cache = true }
```

```yaml
# .github/workflows/ci.yml
- uses: iPeluwa/rx@v1
  with:
    command: build --release
    profile: ci
  env:
    AWS_ACCESS_KEY_ID: ${{ secrets.AWS_ACCESS_KEY_ID }}
    AWS_SECRET_ACCESS_KEY: ${{ secrets.AWS_SECRET_ACCESS_KEY }}
```

## Cache invalidation

The remote cache uses the same fingerprint as the local cache. A new fingerprint is generated whenever source files, `Cargo.toml`, `Cargo.lock`, the build profile, or RUSTFLAGS change. Old artifacts are not automatically cleaned from remote storage -- use your storage provider's lifecycle policies for that.

## Disabling remote cache

Remove or empty the `remote_cache` field:

```toml
[build]
remote_cache = ""
```
