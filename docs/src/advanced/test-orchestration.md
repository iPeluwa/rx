# Smart Test Orchestration

rx provides intelligent test execution through the `rx test-smart` command, which uses failure history, timing data, and sharding to optimize test runs.

## Failure-based ordering

`rx test-smart` maintains a persistent history of test results at `~/.rx/test-history.json`. Tests are ordered by:

1. **Previously-failing tests first** -- tests that failed recently run first to catch regressions immediately
2. **Fastest tests first** (within passing tests) -- provides quick feedback on the majority of tests
3. **Failure count decay** -- when a previously-failing test passes, its failure count is decayed rather than immediately reset, so intermittently-failing tests stay near the top

```sh
rx test-smart                     # run with failure-based ordering
rx test-smart --release           # release mode
```

## Sharding

Distribute tests across multiple parallel runners:

```sh
rx test-smart --shards 4
```

Sharding splits the test suite into N balanced subsets based on historical run times. Each shard gets a roughly equal share of total test time, not just test count.

### CI sharding example

In a GitHub Actions matrix:

```yaml
strategy:
  matrix:
    shard: [1, 2, 3, 4]
steps:
  - uses: iPeluwa/rx@v1
    with:
      command: test-smart --shards 4 --shard-index ${{ matrix.shard }}
```

## Flaky test detection

rx tracks test outcomes over time. A test that alternates between pass and fail across consecutive runs is flagged as flaky. With `--verbose`, flaky tests are annotated in the output:

```
PASS  test_connection (flaky: 3 of last 10 runs failed)
```

Flaky detection helps identify unreliable tests that may need attention.

## Test history

The test history file (`~/.rx/test-history.json`) stores:

- Last N results for each test (pass/fail)
- Average run time per test
- Failure counts with decay

The history is updated after every `rx test-smart` run. It persists across sessions so ordering improves over time.

Clear the history to start fresh:

```sh
rm ~/.rx/test-history.json
```

## Affected-only testing

Combine smart ordering with affected-only testing:

```sh
rx test --affected --base main
```

This first identifies which packages have changed (via `git diff`), then only runs tests for those packages. In a workspace, unchanged packages are skipped entirely.

## Comparison with rx test

| Feature | `rx test` | `rx test-smart` |
|---------|-----------|-----------------|
| Basic test execution | Yes | Yes |
| nextest support | Yes | Yes |
| Affected-only | Yes | No (use `rx test --affected`) |
| Failure-based ordering | No | Yes |
| Sharding | No | Yes |
| Flaky detection | No | Yes |
| History tracking | No | Yes |

Use `rx test` for simple test runs and CI. Use `rx test-smart` when you want intelligent ordering and sharding.
