# **📅 7-Day MVP Plan — Time-Travel Debug Logger**

## **🎯 Goal**

By the end of 7 days, you will have:

1. A Rust crate `ttlog` that records events into a ring buffer.
2. A panic hook that dumps the last N seconds of events to disk.
3. A CLI tool `ttlog-view` that reads and prints those events.

---

## **Day 1 — Project Setup & Planning**

**Tasks:**

* Create a new Rust workspace with:

  ```
  ttlog/         # Core library
  ttlog-view/    # CLI viewer
  examples/      # Example Rust apps using ttlog
  ```
* Set up `Cargo.toml` for both crates.
* Pick dependencies:

  * `serde`, `serde_cbor` (binary encoding)
  * `tracing` (for events)
  * `parking_lot` (fast locks, if needed)
  * `chrono` or `time` (timestamps)
  * `lz4` (compression)
* Decide buffer defaults:

  * Size: **10 MB**
  * Time window: **5 seconds pre-panic**
* Plan file naming convention:

  ```
  /tmp/ttlog-<pid>-<timestamp>.bin
  ```

**Deliverable:** Empty crate + dependencies + README with scope.

---

## **Day 2 — Implement Basic Ring Buffer**

**Tasks:**

* Implement a **single-threaded** fixed-size circular buffer for events.
* Event struct:

  ```rust
  #[derive(Serialize, Deserialize)]
  struct Event {
      ts: u64,
      level: u8,
      message: String,
  }
  ```
* Support `push(event)` and `iter()` to read in order.
* Add a test that pushes 100 events into a buffer of size 10 and confirms only the last 10 remain.

**Deliverable:** Passing tests for ring buffer logic.

---

## **Day 3 — Add `tracing` Integration**

**Tasks:**

* Create a `Layer` for `tracing` that writes each log event into the ring buffer.
* Support:

  * Timestamp
  * Log level
  * Message
* Add an `init()` function to install your tracing layer.

**Deliverable:** Example app that logs messages and can `println!` the buffer contents.

---

## **Day 4 — Add Panic Hook & Snapshot Writing**

**Tasks:**

* Use `std::panic::set_hook` to detect panics.
* On panic:

  * Read last N seconds from buffer.
  * Serialize to CBOR.
  * Compress with LZ4.
  * Write file to `/tmp/ttlog-<pid>-<timestamp>.bin`.
* Ensure panic hook doesn’t panic itself.
* Add a manual function `flush_snapshot(reason: &str)` for testing.

**Deliverable:** Example app that panics and produces a `.bin` file.

---

## **Day 5 — Implement CLI Viewer**

**Tasks:**

* Create binary `ttlog-view`:

  ```
  ttlog-view /tmp/ttlog-12345-2025-08-13.bin
  ```
* Reads file:

  * Decompress LZ4.
  * Decode CBOR.
  * Pretty-print:

    ```
    12:34:56.123 [INFO] user logged in
    12:34:56.200 [ERROR] db connection failed
    ```
* Add colored output by log level.

**Deliverable:** CLI prints events from panic snapshot.

---

## **Day 6 — Add Config & Thread Safety**

**Tasks:**

* Support configurable:

  * Buffer size (MB)
  * Pre-window duration
* Make buffer **per-thread** to avoid contention:

  * Use `thread_local!` for buffers.
  * Add a global snapshot collector that merges them.
* Benchmark with `cargo bench` to ensure low overhead.

**Deliverable:** Multi-threaded support + basic benchmarks.

---

## **Day 7 — Polish & Document**

**Tasks:**

* Write full README:

  * Installation
  * Usage (Rust example)
  * CLI usage
* Add license.
* Record a short GIF demo of:

  * Running example app.
  * Causing a panic.
  * Viewing snapshot in CLI.
* Push to GitHub.

**Deliverable:** Public repo ready for feedback.

---

# **MVP Architecture**

**Rust App**
↓ (events via tracing)
**Ring Buffer (per-thread)**
↓ (on panic)
**Snapshot Writer** (CBOR → LZ4 → file)
↓ (view later)
**CLI Viewer (`ttlog-view`)**
