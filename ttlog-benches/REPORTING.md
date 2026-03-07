# Benchmark Reporting Policy

This benchmark suite is intended for engineering decisions, not marketing claims.

## Required metadata
- UTC run timestamp
- OS and architecture
- CPU parallelism (logical)
- Build profile (`debug` or `release`)
- Per-trial duration and run count

## Validity rules
- Clearly label synthetic tests vs real-path tests.
- Count overwrite events as drops where data loss semantics apply.
- Fail file-sink benchmarks when output file creation fails.
- State memory metric scope explicitly (what is included/excluded).

## Output artifact
- Every run writes an audit report to `ttlog-benches/reports/latest.md`.
- The report must contain metadata, integrity policy, and all result tables.

## Reproducibility checklist
- Use `--release` for publishable numbers.
- Run on a stable machine state (minimal background load).
- Keep benchmark duration and run count unchanged between comparisons.
- Optional runtime overrides:
  - `TTLOG_BENCH_DURATION_SECS=<n>` (default `3`)
  - `TTLOG_BENCH_RUNS=<n>` (default `5`)
