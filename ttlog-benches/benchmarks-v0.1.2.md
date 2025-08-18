# TTLog performance comparison — v0.1.0 (image) vs v0.1.2 (text)

**Context:** v0.1.0 numbers were taken from the screenshot you provided; v0.1.2 numbers came from the textual benchmark output. I compare the same metrics side-by-side, show absolute & relative changes, and indicate the winner per metric (higher is better for throughput/ops; lower is better for memory per event).

---

## Executive summary

* **v0.1.0** is clearly superior for **raw throughput**: events/sec and buffer ops/sec are an order of magnitude higher.
* **v0.1.2** achieved a **dramatic memory improvement** (≈104 bytes/event vs \~1,170 bytes/event), but at the cost of throughput.
* Concurrency limits (threads, buffers) are identical between versions.
* **Snapshot performance** is reported for v0.1.0 (988,506 events/sec) and is **missing** from the v0.1.2 report — treat that as unknown for v0.1.2.

---

## Detailed comparison

| Metric                                              |        v0.1.0 |     v0.1.2 | Absolute diff (v0.1.2 - v0.1.0) | v0.1.2 as % of v0.1.0 | Change (%) |                      Winner |
| --------------------------------------------------- | ------------: | ---------: | ------------------------------: | --------------------: | ---------: | --------------------------: |
| Maximum Events per Second                           |  4,741,298.00 | 285,468.45 |                   -4,455,829.55 |                 6.02% |    -93.98% |                  **v0.1.0** |
| Maximum Buffer Operations per Second                | 10,611,142.00 | 904,928.48 |                   -9,706,213.52 |                 8.53% |    -91.47% |                  **v0.1.0** |
| Maximum Concurrent Threads                          |         1,024 |      1,024 |                            0.00 |               100.00% |     +0.00% |         **Tie** (no change) |
| Maximum Concurrent Buffers                          |       100,000 |    100,000 |                            0.00 |               100.00% |     +0.00% |         **Tie** (no change) |
| Memory Efficiency (bytes/event) — *lower is better* |      1,169.80 |     104.00 |                       -1,065.80 |                 8.89% |    -91.11% |                  **v0.1.2** |
| Snapshot Performance (events/sec)                   |    988,506.00 |        N/A |                             N/A |                   N/A |        N/A | **v0.1.0** (v0.1.2 missing) |

**Notes on columns**

* **Absolute diff** = v0.1.2 − v0.1.0 (negative means v0.1.2 is lower).
* **v0.1.2 as % of v0.1.0** shows how big v0.1.2 is relative to v0.1.0 (e.g., 6.02% means v0.1.2 is \~6% of v0.1.0).
* **Change (%)** = relative change (positive = increase, negative = decrease).

---

## Short interpretation & recommended next steps

1. **Investigate build/profile differences**

   * Ensure both benchmarks were run with the **same build profile** (`cargo bench` / `--release`) and identical compiler flags. A debug build or extra instrumentation in v0.1.2 could explain the throughput drop.
2. **Compare runtime/test parameters**

   * Confirm identical test durations, thread counts, buffer sizes and producer/consumer counts. Small differences in harness config can produce large throughput changes.
3. **Check recent code changes**

   * Look at commits between v0.1.0 and v0.1.2 for changes to hot paths (event encoding, synchronization, string interner, allocation strategies).
   * The warnings in the v0.1.2 log (unused imports/variables, missing `main` in benches) suggest the codebase may have diverged or tests compiled differently.
4. **Profile the slow build**

   * Run a CPU/memory profile (perf/pprof) on v0.1.2 under the same test to find hotspots causing the throughput drop.
5. **Re-run snapshots**

   * Re-run snapshot performance for v0.1.2 (it was not reported). Without that, one crucial metric is unknown.
6. **Trade-off decision**

   * Decide whether memory reduction (≈91% less bytes/event) justifies the throughput regression. If the aim is low-memory, v0.1.2 is promising but needs optimization to regain throughput.

