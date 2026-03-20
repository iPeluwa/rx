# Test

rx provides two test commands: `rx test` for standard test runs, and `rx test-smart` for intelligent test orchestration with failure-based ordering and sharding.

## Basic usage

```sh
rx test                          # run all tests
rx test --release                # test in release mode
rx test -- --nocapture           # pass flags to the test harness
rx test -- test_name             # run a specific test
```

## Test runner selection

rx selects the test runner based on your `rx.toml` configuration:

```toml
[test]
runner = "auto"      # use nextest if installed, else cargo test
runner = "nextest"   # always use cargo-nextest
runner = "cargo"     # always use cargo test
extra_args = []      # extra args passed to every test run
```

With `"auto"`, rx checks for `cargo-nextest` on the PATH and uses it when available. nextest provides better output formatting, per-test timeouts, and parallel execution.

## Affected-only testing

Only test packages that have changed since a base ref:

```sh
rx test --affected                # changed since HEAD~1
rx test --affected --base main    # changed since main branch
rx test --affected --base v1.0    # changed since a tag
```

rx maps changed files from `git diff` to workspace members and only runs tests for affected packages. This is especially useful in CI for large workspaces.

## Smart test orchestration

`rx test-smart` uses failure history to order tests intelligently:

```sh
rx test-smart                     # run tests, failed-first ordering
rx test-smart --release           # release mode
rx test-smart --shards 4          # distribute across 4 shards
```

How it works:

1. Previously-failing tests run first (catch regressions immediately)
2. Within passing tests, fastest tests run first (quick feedback)
3. Failure counts decay on success (stale failures fade out)

History is persisted at `~/.rx/test-history.json`.

## Sharding

Distribute tests across parallel runners:

```sh
rx test-smart --shards 4          # split into 4 shards
```

Each shard gets a balanced subset of tests based on historical run times. This is useful in CI to parallelize test execution across multiple runners.

## Flaky test detection

`rx test-smart` tracks test results over time. Tests that alternate between pass and fail are flagged as flaky in the test history. Use `--verbose` to see flaky test annotations in the output.

## Coverage

```sh
rx coverage                       # HTML report
rx coverage --open                # build and open in browser
rx coverage --lcov                # LCOV output (writes lcov.info)
```

rx uses `cargo-llvm-cov` or `tarpaulin`, whichever is available.

## Advanced testing

```sh
rx test-advanced snapshot         # snapshot testing
rx test-advanced fuzz             # fuzz testing
rx test-advanced mutate           # mutation testing
```

## Related commands

- `rx bench` -- run benchmarks
- `rx ci` -- run the full pipeline (includes tests)
- [Smart Test Orchestration](../advanced/test-orchestration.md) -- detailed orchestration docs
