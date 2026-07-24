# Test

`rx test` runs your tests with the best available runner (nextest when installed) and can restrict the run to packages affected by a change.

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

rx maps changed files from `git diff` to workspace members, expands the set to transitive dependents (a change in `core` also affects everything that depends on it), and runs **one** test invocation with repeated `-p` selections. If nothing relevant changed, the run is skipped.

`--affected` also works on `rx ci` and any task: `rx ci --affected`, `rx run lint --affected`. Shell tasks receive the selection as `RX_AFFECTED_PACKAGES`.

## Related commands

- `rx ci` -- run the full pipeline (includes tests)
