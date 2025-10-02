# `ttlog` Crate

The `ttlog` crate is the core engine of the logging framework. It manages the entire lifecycle of a log event, from its creation to its processing by listeners and its storage in the snapshot buffer.

## Core Concepts

### The `Trace` Struct

The `Trace` struct is the central orchestrator. A single `Trace` instance is typically created when the application starts and is stored in a `thread_local!` static variable, making it accessible to the logging macros.

Its key responsibilities include:
- Spawning and managing the **writer thread** and **listener thread**.
- Holding the channels that decouple the main application threads from the logging backend.
- Providing the API for configuration, like adding listeners and setting the log level.

### Initialization: `Trace::init()`

You initialize the entire system by calling `Trace::init()`.

```rust
pub fn init(
    capacity: usize,
    channel_capacity: usize,
    service_name: &str,
    storage_path: Option<&str>,
) -> Self
```

- `capacity`: The number of `LogEvent`s to keep in the in-memory ring buffer for snapshots. A larger capacity means more history is available in a crash dump, but it consumes more memory.
- `channel_capacity`: The size of the bounded channel used for control messages (like "create a snapshot now").
- `service_name`: An identifier for your application, which gets embedded in snapshot files.
- `storage_path`: The directory where snapshot files will be saved. Defaults to `./tmp/`.

### `LogEvent`

This struct is the internal representation of a single log record. It is highly optimized for size and speed, using integer IDs instead of storing strings directly.

```rust
pub struct LogEvent {
  pub packed_meta: u64, // Timestamp, level, and thread ID packed into a u64
  pub target_id: u16,
  pub message_id: Option<num::NonZeroU16>,
  pub kv_id: Option<num::NonZeroU16>,
  pub file_id: u16,
  pub position: (u32, u32), // (line, column)
}
```
All the `_id` fields are integer handles that refer to strings managed by the `StringInterner`.

### Listeners

Listeners are responsible for processing log events in real-time. They run on a dedicated listener thread and receive events as they happen. This is how you get immediate output, for example, to the console or a log file.

The `LogListener` trait is simple:
```rust
pub trait LogListener: Send + Sync + 'static {
  fn handle(&self, event: &LogEvent, interner: &StringInterner);
  // ... other optional methods
}
```
TTLog provides two built-in listeners:
- **`StdoutListener`**: Formats and prints logs to standard output.
- **`FileListener`**: Writes logs to a specified file.

You can easily create your own listener to send logs to a network service, a database, or any other destination.

### Snapshots

A key feature of TTLog is its snapshotting capability. The system maintains an in-memory, lock-free ring buffer that stores the last `N` log events (where `N` is the `capacity` set during `init`). This buffer is not typically consumed by listeners; it exists purely for crash diagnostics.

A snapshot is a dump of the entire contents of this ring buffer, along with metadata about the application. Snapshots are automatically triggered by:
1.  **Panics**: A global panic hook is installed by default. If the application panics, a snapshot is written to disk just before it exits.
2.  **Signals**: The library listens for OS signals like `SIGINT`, `SIGTERM`, `SIGQUIT`, and `SIGSEGV`. A snapshot is written when one of these is received.
3.  **Periodic Trigger**: The writer thread will automatically create a snapshot every 60 seconds if new events have been logged.
4.  **Manual Request**: You can call `trace.request_snapshot("my-reason")` to trigger one programmatically.

Snapshot files are serialized using `serde_cbor` and compressed with `lz4`, making them small and efficient to store and transfer.

### String Interning

To avoid the performance cost of allocating, cloning, and storing duplicate strings (like file paths, module names, or common log messages), TTLog uses a high-performance string interner (`StringInterner`).

When a log macro is called for the first time (e.g., `info!("Starting up")` in `src/main.rs`), the interner stores the strings `"Starting up"`, `"my_app::main"`, and `"src/main.rs"` in a global, thread-safe hash map and assigns a unique integer ID to each.

On subsequent calls, the macro uses the cached ID, which is a nearly free integer lookup. This dramatically reduces memory usage and allocation overhead in applications with repetitive logging. It uses a `thread_local!` cache to make lookups even faster by avoiding lock contention in the common case.
