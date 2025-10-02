# TTLog Documentation

Welcome to the official documentation for TTLog, a high-performance, low-allocation logging framework for Rust.

## Overview

TTLog is designed from the ground up for speed and efficiency, making it ideal for high-throughput applications where logging overhead is a critical concern. It achieves this through a combination of modern Rust features, lock-free data structures, and a decoupled architecture.

### Core Features

- **High-Performance Macros**: `info!`, `debug!`, etc., are procedural macros that perform most of their work at compile time, minimizing runtime cost.
- **String Interning**: All repetitive strings (log messages, file paths, module paths, keys) are interned, storing each unique string only once.
- **Lock-Free Ring Buffer**: Log events are written to a lock-free, in-memory ring buffer, ensuring that logging calls are non-blocking.
- **Decoupled Listeners**: Listeners (e.g., for stdout or file output) run on a separate thread, consuming events from the main logging thread without blocking application code.
- **Automatic Snapshots**: TTLog maintains a ring buffer of recent events and can automatically write a compressed snapshot to disk in critical situations.
- **Crash & Signal Handling**: It automatically captures a snapshot of recent logs when the application panics or receives a termination signal (like `SIGINT` or `SIGTERM`), providing crucial context for post-mortem debugging.

## Crate Structure

The TTLog ecosystem is split into two main crates:

1.  [**`ttlog`**](./ttlog.md): The core logging library. It contains the `Trace` engine, listeners, snapshot functionality, and all the runtime components.
2.  [**`ttlog-macros`**](./ttlog-macros.md): A procedural macro crate that provides the user-facing logging macros (`info!`, `warn!`, etc.).

## Getting Started

Here is a minimal example of how to initialize TTLog and log a simple message.

```rust
use ttlog::trace::{Trace, GLOBAL_LOGGER};
use ttlog_macros::info;
use ttlog::stdout_listener::StdoutListener;
use std::sync::Arc;

fn main() {
    // 1. Initialize the TTLog trace engine
    let mut trace = Trace::init(
        10_000, // In-memory snapshot buffer capacity
        128,    // Bounded channel capacity for control messages
        "my-app", // Service name for snapshots
        Some("./tmp/"), // Path to store snapshots
    );

    // 2. Add a listener to process logs in real-time
    // The StdoutListener prints formatted logs to the console.
    trace.add_listener(Arc::new(StdoutListener::new()));

    // 3. Start logging!
    info!("Application started successfully");
    let user_id = 12345;
    let status = "active";
    info!(user_id = user_id, status = status, "User logged in");

    // 4. Manually shut down the logger (optional, happens on drop)
    // This ensures all buffered events are processed.
    trace.shutdown();
}
```

This example initializes the system, adds a simple listener that prints to the console, and logs two messages. The second message includes structured key-value data.
