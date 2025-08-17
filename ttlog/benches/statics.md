# TTLog — Benchmark Report (Markdown)

## Executive summary

TTLog shows **excellent** raw in-memory throughput and strong concurrency scaling. It sustained **\~4.74M events/sec** in high-frequency tests and **\~10.6M ops/sec** for buffer operations. Snapshot throughput (\~988k events/sec) is impressive for checkpointing. The main downside is a non-trivial **memory overhead ≈ 1,170 bytes/event** — acceptable for medium/large events, suboptimal for many tiny events. Some benchmarks show noise and warnings; re-running long tests with longer durations will make metrics more reliable.

**Verdict:** **A−** — outstanding throughput and concurrency; optimize memory per-event and stabilize benchmarks to reach A+.

---

## Grand summary (compact)

| Metric                               |      Value |        Unit |
| ------------------------------------ | ---------: | ----------: |
| Maximum Events per Second            |  4,741,298 |  events/sec |
| Maximum Buffer Operations per Second | 10,611,142 |     ops/sec |
| Maximum Concurrent Threads           |      1,024 |     threads |
| Maximum Concurrent Buffers           |    100,000 |     buffers |
| Memory Efficiency                    |    1,169.8 | bytes/event |
| Snapshot Performance                 |    988,506 |  events/sec |

---

## Key test table (detailed)

| Test Name             |      Metric |      Value |        Unit | Duration | Additional info                                                 |
| --------------------- | ----------: | ---------: | ----------: | -------: | --------------------------------------------------------------- |
| High Frequency Events |  Events/sec |  4,741,298 |  events/sec |  10.01 s | Total events: 47,469,359 · threads: 16 · buffer size: 1,000,000 |
| Buffer Operations     |     Ops/sec | 10,611,142 |     ops/sec |  10.12 s | Total ops: 107,423,878 · producers: 8 · consumers: 4            |
| Concurrent Threads    | Max threads |      1,024 |     threads | 69.36 ms | Per-thread latencies (1→1024): up to 34.94 ms                   |
| Concurrent Buffers    | Max buffers |    100,000 |     buffers |   3.25 s | Times for 1..100k buffers (2.8006s for 100k)                    |
| Memory Efficiency     | Bytes/event |   1,169.78 | bytes/event |   109 ms | Total memory ≈ 129,962,360 bytes for 111,100 events             |
| Snapshot Performance  |  Events/sec | 988,505.83 |  events/sec |   112 ms | 3 snapshots, 111,000 events total                               |

---

## Other highlights (microbenchmarks & stress)

* Buffer microbench (100k items): push **24,291,000 ops/sec**, pop **30,251,548 ops/sec**, mixed **18,081,548 ops/sec**.
* Event creation: direct **8,345,571 ev/sec**, builder **3,001,554 ev/sec**, with fields **4,448,167 ev/sec**.
* Logging: single thread **1,018,050 logs/sec**, 4 threads **2,835,140 logs/sec**.
* Distributed sim: cache hit rate **96.67%**, node ops up to **\~593k ops/sec**; microservice sim shows slow processing (20.64 req/sec) due to simulated blocking behavior.
* Stress tests: heavy CPU/network/memory tests completed successfully; memory fragmentation test ran 1000 iterations (\~91.65s).

---

## Analysis — what the numbers mean

**Throughput**

* 4.74M events/sec and 10.6M buffer ops/sec show TTLog is extremely fast for *moving events in memory*. These numbers are in the realm of high-performance, lock-optimized queues.

**Concurrency**

* Supporting 1,024 threads and 100k buffers (without crashing) demonstrates robust multi-threaded design. Latency grows but remains reasonable (tens of ms at extreme scale).

**Serialization & snapshot differences**

* Lower “max” numbers (e.g., \~980k events/sec) come from snapshot / serialization tests — serialization and I/O are expected to cost CPU and reduce throughput. Compare like-for-like: in-memory vs. serialized workloads differ widely.

**Memory per-event**

* \~1.17 KB/event is the largest concern. If your typical event payload is small (<100 bytes), this overhead dominates and could be reduced by:

  * pooling / arenas,
  * fewer heap allocations,
  * compact metadata layout.

**Benchmark reliability**

* Many bench logs show warnings: “Unable to complete 100 samples in 5.0s” and non-trivial outlier counts (some tests up to 18% outliers). These indicate noisy measurements; increase bench durations and warmups for more reliable results.

---

## Recommendations — immediate, practical

1. **Profile memory**: run a heap profiler (heaptrack, massif, jemalloc profiling) on a representative test (e.g., 1M events) to locate per-event allocations (strings, Vec growth, boxed fields).
2. **Add object pooling/arena** for event structures to cut per-event allocation overhead.
3. **Stabilize benchmarks**:

   * Increase bench target times per the tool warnings.
   * Increase warmup time.
   * Run on isolated hardware (CPU governor fixed, no background jobs).
4. **Collect latency distributions**: measure P50/P95/P99/P999 for push/pop and logging, not just throughput.
5. **Separate micro vs macro benchmarks**:

   * Microbench: raw push/pop, minimal payload.
   * Macrobench: real-world payloads + serialization + snapshot.
6. **Benchmark serialization strategies**: compare CBOR/JSON/CBOR streaming/flatbuffers to see cost for serialized snapshots.
7. **CI performance monitoring**: use criterion/bench harness in CI to detect regressions and store baselines.

---

## Risk & caveats

* **Memory overhead** could be prohibitive at scale for tiny-event workloads.
* **Noisy measurements** reduce confidence in some numbers — re-run long tests.
* Some distributed/microservice simulations show low throughput (20.64 req/sec) because those tests model slow processing; don't conflate that with logging performance.
