# TTLog - High-Performance Structured Logging for Rust

[![Crates.io](https://img.shields.io/crates/v/ttlog)](https://crates.io/crates/ttlog)
[![Documentation](https://docs.rs/ttlog/badge.svg)](https://docs.rs/ttlog)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

TTLog is a structured logging library designed for high-throughput Rust applications. It uses lock-free ring buffers and efficient serialization to minimize performance impact while providing comprehensive logging capabilities and crash-safe snapshot generation.

## Performance Characteristics

Recent benchmark results from our test suite (16-core system, averaged over 5 runs):

```
Throughput Benchmarks:
┌─────────────────────────────┬─────────────────┬─────────────┬──────────────┐
│ Configuration               │ Events/sec      │ Std Dev     │ Buffer Size  │
├─────────────────────────────┼─────────────────┼─────────────┼──────────────┤
│ 16 threads, 1KB buffer     │ 317.8M ±3.2M    │ 1.0%        │ 1,024        │
│ 16 threads, 8KB buffer     │ 304.0M ±4.7M    │ 1.5%        │ 8,192        │
│ 16 threads, 64KB buffer    │ 300.0M ±1.3M    │ 0.4%        │ 65,536       │
└─────────────────────────────┴─────────────────┴─────────────┴──────────────┘

Memory Efficiency:
┌─────────────────────────────┬─────────────────┬─────────────────────────────┐
│ Metric                      │ Value           │ Notes                       │
├─────────────────────────────┼─────────────────┼─────────────────────────────┤
│ Bytes per event             │ 24 bytes        │ Core event structure        │
│ Memory allocations/sec      │ 2.16M ±15K     │ Mostly string interning     │
│ Memory throughput           │ 757 MB/sec      │ Including serialization     │
└─────────────────────────────┴─────────────────┴─────────────────────────────┘

Concurrency:
┌─────────────────────────────┬─────────────────┬─────────────────────────────┐
│ Test                        │ Result          │ Configuration               │
├─────────────────────────────┼─────────────────┼─────────────────────────────┤
│ Maximum concurrent threads  │ 256 threads     │ 0.9s runtime               │
│ Maximum concurrent buffers  │ 1,000 buffers   │ 100 ops/buffer             │
│ Producer-heavy (8P/4C)      │ 14.8M ops/sec   │ 1KB buffer                 │
└─────────────────────────────┴─────────────────┴─────────────────────────────┘
```

**Key Technical Points:**
- Lock-free ring buffers using `crossbeam::ArrayQueue`
- Thread-local string interning with fallback to shared cache
- Structured data serialization via serde + CBOR
- Configurable snapshot generation (panic hooks, periodic, manual)
- Bounded memory usage with overflow protection

## Architecture

TTLog separates logging into three phases:

1. **Event Creation** (application thread): Minimal work, mostly lock-free
2. **Buffering** (ring buffer): Lock-free push with overflow handling  
3. **Processing** (writer thread): Serialization, compression, and I/O

```
Application Thread          Writer Thread               Storage
─────────────────          ─────────────               ─────────
┌─────────────────┐        ┌─────────────────┐         ┌──────────────┐
│ Log macro call  │───────▶│ Ring buffer     │────────▶│ CBOR + LZ4   │
│ String interning│        │ (lock-free)     │         │ Atomic write │
│ Event creation  │        │ Batch processing│         │              │
└─────────────────┘        └─────────────────┘         └──────────────┘
```

The design prioritizes:
- **Predictable performance**: Bounded memory usage, consistent latency
- **Non-blocking operation**: Applications never wait for I/O
- **Data integrity**: Atomic snapshots, crash recovery via panic hooks
- **Observability**: Rich structured data with efficient storage

## Quick Start

Add to `Cargo.toml`:
```toml
[dependencies]
ttlog = "0.1.0"
```

Basic usage:
```rust
use ttlog::trace::Trace;
use ttlog::ttlog_macros::{info, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize: (buffer_size, channel_size, service_name, storage_path)
    let _trace = Trace::init(4096, 64, "my-service", Some("./logs"));

    // Structured logging
    info!("Server starting", port = 8080, workers = 4);
    
    // Key-value pairs are serialized efficiently
    for request_id in 0..1000 {
        info!("Processing request", 
              id = request_id, 
              user_id = 12345, 
              duration_ms = 42);
    }

    // Snapshots are created automatically on panic
    // Manual snapshots can be requested:
    // trace.request_snapshot("checkpoint");

    Ok(())
}
```

## Configuration

### Buffer Sizing

Choose buffer sizes based on your throughput requirements:

```rust
// High-frequency applications (trading, real-time systems)
let trace = Trace::init(100_000, 10_000, "trading-system", Some("./logs"));

// Web services (moderate load)
let trace = Trace::init(10_000, 1_000, "web-api", Some("./logs"));

// Embedded/resource-constrained
let trace = Trace::init(1_000, 100, "iot-device", Some("./logs"));
```

### Log Levels

```rust
use ttlog::event::LogLevel;

// Set minimum log level (atomic operation)
trace.set_level(LogLevel::WARN);

// Check current level
let current = trace.get_level();
```

## Advanced Features

### Custom Listeners

TTLog supports pluggable output destinations:

```rust
use ttlog::stdout_listener::init_stdout;
use ttlog::file_listener::init_file;

// Console output
init_stdout()?;

// File output  
init_file("application.log")?;

// Custom listener implementation
impl LogListener for MyCustomListener {
    fn handle(&self, event: &LogEvent, interner: &StringInterner) {
        // Process events with minimal allocation
    }
}
```

### Snapshot Analysis

Snapshots are compressed CBOR files that can be analyzed:

```bash
# Install the viewer (when available)
cargo install ttlog-view

# View snapshots
ttlog-view logs/ttlog-*.bin --filter level=ERROR
```

## Performance Comparison

Informal comparisons with other Rust logging libraries on similar hardware:

| Library | Approx. Throughput | Memory/Event | Blocking Risk |
|---------|-------------------|--------------|---------------|
| TTLog | 300M+ events/sec | 24-40 bytes | None (lock-free) |
| tracing | ~10-20M events/sec | ~100-200 bytes | Low (some contention) |
| slog | ~5-15M events/sec | ~80-150 bytes | Medium (mutex-based) |
| env_logger | ~1-3M events/sec | ~200-400 bytes | High (synchronous I/O) |

*Note: These are rough estimates. Performance varies significantly with workload patterns, hardware, and configuration. Always benchmark with your specific use case.*

## Production Considerations

**Memory Management:**
- Ring buffers have fixed capacity - events are dropped when full
- String interning reduces memory usage but adds computational overhead
- Monitor buffer utilization in production systems

**Error Handling:**
- Panic hooks automatically create snapshots
- Manual snapshots can be triggered for debugging
- Failed I/O operations are logged to stderr but don't block logging

**Thread Safety:**
- All public APIs are thread-safe
- Ring buffer operations are lock-free
- String interning uses thread-local caches with shared fallback

## Limitations

- **Fixed buffer capacity**: No dynamic resizing (by design)
- **Best-effort delivery**: Events may be dropped under extreme load
- **Single writer thread**: All I/O goes through one background thread
- **Platform dependencies**: Requires crossbeam-compatible atomics

## Contributing

Contributions welcome. Please ensure:
- Tests pass: `cargo test`
- Benchmarks don't regress: `cargo bench`
- Code is formatted: `cargo fmt`

## License

MIT License - see [LICENSE](LICENSE) file.

## Acknowledgments

- [crossbeam](https://github.com/crossbeam-rs/crossbeam) for lock-free data structures
- [serde](https://serde.rs) for efficient serialization
- [lz4](https://lz4.github.io/lz4/) for fast compression

---

TTLog aims to provide predictable, high-performance logging for systems where throughput and low latency matter. It trades some flexibility for performance characteristics that are well-suited to high-frequency applications.
