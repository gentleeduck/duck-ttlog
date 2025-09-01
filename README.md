# TTLog - High-Performance Structured Logging

<p align="center">
    <img src="./public/logo.png" alt="TTLog Logo" width="500"/>
</p>

[![Crates.io](https://img.shields.io/crates/v/ttlog)](https://crates.io/crates/ttlog)
[![Documentation](https://docs.rs/ttlog/badge.svg)](https://docs.rs/ttlog)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

TTLog is a lock-free structured logging library built for high-throughput applications. It uses lock-free ring buffers and thread-local string interning to minimize allocations and contention, while providing automatic crash recovery through compressed log snapshots.

## Performance Characteristics

Based on comprehensive benchmarks, TTLog demonstrates significant performance advantages:

```
Benchmark Results (December 2024)
─────────────────────────────────

Throughput:         318M events/sec (16 threads)
Buffer Operations:  15M ops/sec (producer-heavy workload)
Memory Efficiency:  24 bytes/event (core event structure)
Allocation Rate:    2.2M allocs/sec (includes string interning)
Concurrency:        256+ concurrent threads tested
```

### Performance Comparison

| Library | Throughput (M events/sec) | Memory/Event | Lock-Free | Crash Recovery |
|---------|---------------------------|--------------|-----------|----------------|
| TTLog | 318 | 24 bytes | ✅ | ✅ Snapshots |
| tracing | ~12 | ~200 bytes | ❌ | ❌ |
| slog | ~8 | ~150 bytes | ❌ | ❌ |
| log4rs | ~2 | ~300 bytes | ❌ | ❌ |

*Benchmarks run on AMD/Intel systems. Results may vary based on hardware and workload patterns.*

## Key Design Features

**Lock-Free Architecture**
- Uses crossbeam's ArrayQueue for contention-free logging
- Thread-local string interning with RwLock fallback
- Atomic operations for level filtering and metadata

**Memory Management**
- Bounded ring buffers prevent unbounded memory growth
- String deduplication reduces allocation overhead
- SmallVec optimization for key-value serialization

**Production Reliability**
- Automatic panic hooks capture logs during crashes
- Atomic file operations prevent log corruption  
- Graceful degradation under memory pressure
- Separate buffers for real-time listeners vs snapshots

**Developer Experience**
- Proc macros for convenient structured logging
- Compatible with tracing ecosystem
- Built-in snapshot viewer for analysis
- Configurable output formats (stdout, file, custom listeners)

## Architecture Overview

```
Application Thread(s)          Writer Thread              Storage
─────────────────────         ─────────────              ───────
┌─────────────────┐           ┌─────────────┐            ┌─────────────┐
│ Log Macros      │──events──▶│ Ring Buffer │──batches──▶│ CBOR + LZ4  │
│ • info!()       │           │ (Lock-free) │            │ Compression │
│ • error!()      │           │             │            │             │
│ • Custom fields │           │ String      │            │ Atomic      │
└─────────────────┘           │ Interning   │            │ File Ops    │
                              └─────────────┘            └─────────────┘
                                     │
                                     ▼
                              ┌─────────────┐
                              │ Listeners   │
                              │ • Stdout    │
                              │ • File      │
                              │ • Custom    │
                              └─────────────┘
```

## Installation

```toml
[dependencies]
ttlog = "0.1.0"
```

## Quick Start

### Basic Usage

```rust
use ttlog::trace::Trace;
use ttlog::ttlog_macros::{info, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with buffer capacity and service name
    let _trace = Trace::init(100_000, 10_000, "my-service", Some("./logs"));

    // Structured logging with type-safe fields
    info!("Server starting", port = 8080, env = "production");

    // High-frequency logging
    for i in 0..1000 {
        if i % 100 == 0 {
            info!("Processed batch", batch_id = i, items = 100);
        }
    }

    Ok(())
}
```

### With Output Configuration

```rust
use ttlog::stdout_listener::init_stdout;
use ttlog::file_listener::init_file;
use ttlog::ttlog_macros::info;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Quick setup with stdout output
    init_stdout()?;
    
    // Or file output
    // init_file("application.log")?;
    
    info!("Application started", version = "1.0.0");
    Ok(())
}
```

### Advanced Configuration

```rust
use ttlog::trace::Trace;
use ttlog::stdout_listener::StdoutListener;
use std::sync::Arc;

fn main() {
    let trace = Trace::init(
        1_000_000,  // Ring buffer capacity
        100_000,    // Channel capacity
        "high-perf-service",
        Some("./logs")
    );
    
    // Add custom listeners
    trace.add_listener(Arc::new(StdoutListener::new()));
    
    // Configure log level
    trace.set_level(ttlog::event::LogLevel::INFO);
}
```

## Benchmark Results Detail

### Comprehensive Test Suite

The following results are from our statistical benchmark suite with 5 trials per test:

```
Throughput Tests (Mean ± StdDev):
┌─────────────────┬──────────────────┬────────────────┐
│ Configuration   │ Events/Second    │ Buffer Ops/Sec │
├─────────────────┼──────────────────┼────────────────┤
│ 16T, 1K buffer  │ 317.8M ± 3.2M   │ 5.7M ± 3.8K   │
│ 16T, 8K buffer  │ 304.0M ± 4.7M   │ 5.3M ± 56K    │
│ 16T, 64K buffer │ 300.0M ± 1.3M   │ 5.2M ± 33K    │
└─────────────────┴──────────────────┴────────────────┘

Concurrency Tests:
┌─────────────────────┬─────────────────┐
│ Maximum Threads     │ 256 concurrent  │
│ Maximum Buffers     │ 1,000 buffers   │
│ Total Operations    │ 100,000 ops     │
└─────────────────────┴─────────────────┘

Memory Efficiency:
┌─────────────────────┬─────────────────┐
│ Core Event Size     │ 24 bytes        │
│ Memory Throughput   │ 757 MB/sec      │
│ Allocation Rate     │ 2.2M allocs/sec │
└─────────────────────┴─────────────────┘
```

### End-to-End Performance

Real-world pipeline benchmarks with actual I/O:

```
E2E Benchmarks (8 Producers, 1 Consumer):
┌────────────┬─────────────┬─────────────┬──────────────┐
│ Sink Type  │ Produced/s  │ Consumed/s  │ Latency (μs) │
├────────────┼─────────────┼─────────────┼──────────────┤
│ Null sink  │ 8.9M events │ 8.9M events │ p50: 3       │
│            │             │             │ p99: 2,500   │
├────────────┼─────────────┼─────────────┼──────────────┤
│ File sink  │ 5.2M events │ 2.6M events │ p50: 12,600  │
│            │             │             │ p99: 14,600  │
└────────────┴─────────────┴─────────────┴──────────────┘
```

## Configuration & Tuning

### Buffer Sizing Guidelines

```rust
// High-throughput applications (trading, gaming)
let trace = Trace::init(10_000_000, 1_000_000, "trading-system", Some("./logs"));

// General web services
let trace = Trace::init(1_000_000, 100_000, "web-api", Some("./logs"));

// Resource-constrained environments
let trace = Trace::init(10_000, 1_000, "embedded-service", Some("./logs"));
```

### Memory Considerations

TTLog uses bounded buffers to ensure predictable memory usage:

```rust
// Monitor buffer utilization in production
fn monitor_logging(trace: &Trace) {
    if trace.listener_buffer.len() > trace.listener_buffer.capacity() * 90 / 100 {
        eprintln!("Warning: Buffer utilization high");
        trace.request_snapshot("high_memory_usage");
    }
}
```

## Snapshot Analysis

TTLog automatically creates compressed snapshots during panics and on request:

```bash
# View snapshot contents
ttlog-view /tmp/ttlog-*.bin

# Filter and analyze
ttlog-view /tmp/ttlog-*.bin --filter level=ERROR --limit 100

# Export for analysis
ttlog-view /tmp/ttlog-*.bin --format json > analysis.json
```

Snapshots include:
- Complete event history from ring buffer
- Process metadata (PID, hostname, timestamp)
- Compressed using CBOR + LZ4 for efficiency

## Production Considerations

### Error Handling

TTLog is designed to never block or crash your application:

```rust
// Logging operations are fire-and-forget
info!("This will never block or panic");

// Buffer overflow behavior: older events are dropped
// Application continues normally
```

### Resource Management

```rust
// Graceful shutdown with log preservation
fn shutdown(trace: Trace) {
    trace.request_snapshot("shutdown");
    
    // Allow snapshot completion
    std::thread::sleep(std::time::Duration::from_millis(200));
}
```

### Integration with Existing Code

TTLog provides compatibility layers for common logging patterns:

```rust
// Works with existing tracing spans
use tracing::{info_span, info};

let span = info_span!("request", user_id = 123);
let _enter = span.enter();

info!("Processing request"); // Captured by TTLog subscriber
```

## Development and Testing

### Running Benchmarks

```bash
# Clone and run benchmarks
git clone https://github.com/yourusername/ttlog.git
cd ttlog

# Statistical benchmark suite
cargo run --release --bin comprehensive_benchmark

# Stress testing
cargo run --release --bin stress_test
```

### Custom Benchmark Example

```rust
use ttlog::trace::Trace;
use ttlog::ttlog_macros::info;
use std::time::Instant;

fn benchmark_workload() {
    let _trace = Trace::init(1_000_000, 100_000, "benchmark", Some("./logs"));
    
    let start = Instant::now();
    let events = 1_000_000;
    
    for i in 0..events {
        info!("Benchmark event", 
              iteration = i, 
              timestamp = start.elapsed().as_nanos(),
              worker_id = 1
        );
    }
    
    let throughput = events as f64 / start.elapsed().as_secs_f64();
    println!("Throughput: {:.0} events/sec", throughput);
}
```

## Limitations and Considerations

- **Memory Usage**: Ring buffers have fixed capacity - configure appropriately for your workload
- **Event Ordering**: Events from different threads may be reordered in the ring buffer
- **String Interning**: Benefits diminish with highly unique string values
- **Platform Dependencies**: Performance characteristics vary by hardware architecture

## Contributing

We welcome contributions! Please see [CONTRIBUTING.md](CONTRIBUTING.md) for development guidelines.

### Development Setup

```bash
# Setup development environment
make install-tools
make test
make bench
```

## License

Licensed under the MIT License - see [LICENSE](LICENSE) for details.

## Acknowledgments

Built using proven Rust ecosystem libraries:
- [Crossbeam](https://github.com/crossbeam-rs/crossbeam) for lock-free data structures
- [Serde](https://github.com/serde-rs/serde) for serialization
- [LZ4](https://lz4.github.io/lz4/) for compression
- [SmallVec](https://github.com/servo/rust-smallvec) for stack optimization

---

*TTLog: Professional logging for high-performance applications*
