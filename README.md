# TTLog - High-Performance Distributed Logging Library

<p align="center">
  <img src="./public/logo.png" alt="TTLog Logo" width="500"/>
</p>

[![Crates.io](https://img.shields.io/crates/v/ttlog)](https://crates.io/crates/ttlog)
[![Documentation](https://docs.rs/ttlog/badge.svg)](https://docs.rs/ttlog)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

## 🌟 Overview

TTLog is a **high-performance, distributed, non-blocking logging library** for Rust applications that maintains lock-free ring buffers of log events in memory and automatically creates compressed snapshots to disk. It's designed for production systems where logging performance is critical and post-mortem debugging capabilities are essential.

## 🚀 Key Features

### **Performance & Scalability**
- **🔒 Lock-Free Ring Buffer**: Uses crossbeam's battle-tested ArrayQueue for maximum concurrency
- **⚡ Non-Blocking Logging**: Uses crossbeam channels with `try_send` to prevent blocking
- **🌐 Distributed Ready**: Designed for multi-node, multi-threaded distributed systems
- **📈 High Throughput**: Benchmarked at **7.6M events/second** and **16.7M buffer operations/second** on modern systems

### **Reliability & Recovery**
- **🛡️ Automatic Snapshots**: Creates compressed snapshots on panics, periodic intervals, or manual requests
- **💾 Atomic File Operations**: Ensures snapshot files are written atomically to prevent corruption
- **🔄 Crash Recovery**: Automatic panic hook ensures logs are captured during crashes
- **🎯 Zero Data Loss**: Ring buffer ensures recent events are always preserved

### **Integration & Ecosystem**
- **🔗 Tracing Integration**: Implements tracing-subscriber layers for seamless integration
- **📊 Structured Logging**: Rich field support with type-safe structured data
- **🎨 Viewer Tool**: Built-in `ttlog-view` for analyzing and visualizing log snapshots
- **🔧 Proc Macros**: `ttlog-event` crate provides convenient logging macros

### **Storage & Compression**
- **🗜️ Efficient Storage**: CBOR serialization + LZ4 compression for optimal size/speed balance
- **📁 Flexible Output**: Configurable output directories and file naming
- **🔍 Rich Metadata**: Hostname, PID, timestamp, reason, and structured fields

## 🏗️ Architecture

### **Event Processing Pipeline**

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Application   │    │  EventBuilder +  │    │  Writer Thread  │    │   Snapshot      │
│                 │    │ StringInterner   │    │                 │    │   Creation      │
│  tracing::info! │───▶│                  │───▶│ Lock-Free Ring  │───▶│                 │
│  ttlog::event!  │    │ • Field Capping │    │     Buffer      │    │ CBOR + LZ4 +    │
│                 │    │ • String Intern │    │ • 1M+ capacity  │    │ Atomic Write    │
└─────────────────┘    └──────────────────┘    └─────────────────┘    └─────────────────┘
                                ▲                        │                        │
                                │                        ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
                       │ Static Interner │    │ Concurrent Ops  │    │  Compressed     │
                       │ (Thread-Safe)   │    │ 16.7M ops/sec   │    │  Log Files      │
                       │                 │    │ 1024 threads    │    │ 136 bytes/event │
                       └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### **Performance Characteristics**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        TTLog Performance Metrics                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🚀 Throughput:          7.6M events/sec    │  Memory:     136 bytes/event   │
│ ⚡ Buffer Ops:         16.7M ops/sec       │  Allocation: 3.2M allocs/sec   │
│ 🔄 Concurrency:        1,024 threads       │  Throughput: 1.1 GB/sec       │
│ 💾 Buffers:            100K concurrent     │  Efficiency: Lock-free design  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### **Workspace Structure**

```
ttlog/
├── ttlog/              # Main library crate
│   ├── event/          # Log event structures and builders
│   ├── lf_buffer/      # Lock-free ring buffer implementation
│   ├── snapshot/       # Snapshot creation and persistence
│   ├── trace/          # Main orchestration and writer thread
│   ├── trace_layer/    # Tracing-subscriber integration
│   └── panic_hook/     # Crash recovery mechanisms
├── ttlog-event/        # Proc macros for convenient logging
├── ttlog-view/         # Viewer and analysis tool
└── examples/           # Comprehensive usage examples
    ├── ttlog-simple/   # Basic usage patterns
    ├── ttlog-server/   # Server-side logging examples
    ├── ttlog-complex/  # Async and distributed scenarios
    └── ttlog-filereader/ # Snapshot reading examples
```

## 📦 Installation

Add to your `Cargo.toml`:

```toml
[dependencies]
ttlog = "0.1.0"
ttlog-event = "0.1.0"  # Optional: for convenient macros
```

## 🚀 Quick Start

### **Basic Setup**

```rust
use ttlog::trace::Trace;
use tracing::info;

fn main() {
    // Initialize with 10,000 event capacity and 1,000 channel capacity
    let trace = Trace::init(10_000, 1_000);
    
    // Install panic hook for crash recovery
    ttlog::panic_hook::PanicHook::install(trace.get_sender());
    
    // Your application code
    info!("Application started");
    
    // Logs are automatically captured and periodically flushed
}
```

### **With Structured Logging**

```rust
use ttlog::event::{EventBuilder, LogLevel, FieldValue};
use tracing::info;

fn main() {
    let trace = Trace::init(10_000, 1_000);
    
    // Create structured log event
    let event = EventBuilder::new_with_capacity(4)
        .level(LogLevel::Info)
        .target("my_service")
        .message("User action performed")
        .field("user_id", FieldValue::U64(12345))
        .field("action", FieldValue::Str("login".into()))
        .field("ip_address", FieldValue::Str("192.168.1.1".into()))
        .field("success", FieldValue::Bool(true))
        .build();
    
    // Log the event
    info!(target: "my_service", "User action performed", 
          user_id = 12345, action = "login", ip_address = "192.168.1.1", success = true);
}
```

### **Manual Snapshots**

```rust
// Request immediate snapshot
trace.request_snapshot("checkpoint");

// This will create: /tmp/ttlog-{pid}-{timestamp}-checkpoint.bin
```

## 🔧 Configuration

### **Performance Tuning**

```rust
// High-performance configuration for distributed systems
let trace = Trace::init(
    1_000_000,  // 1M events in ring buffer
    10_000,     // 10K events in channel buffer
);

// Conservative configuration for resource-constrained environments
let trace = Trace::init(
    10_000,     // 10K events in ring buffer
    1_000,      // 1K events in channel buffer
);
```

### **Snapshot Configuration**

```rust
// Custom snapshot directory
std::env::set_var("TTLOG_SNAPSHOT_DIR", "/var/log/ttlog");

// Custom service name
std::env::set_var("TTLOG_SERVICE_NAME", "my-microservice");
```

## 📊 Performance Characteristics

### **Throughput Benchmarks**

| System Type | Events/sec | Concurrent Threads | Buffer Capacity |
|-------------|------------|-------------------|-----------------|
| High-End (32+ cores, 64GB+ RAM) | 500K - 2M | 256 - 1024 | 100K - 1M |
| Mid-Range (8-16 cores, 16-32GB RAM) | 100K - 500K | 64 - 256 | 10K - 100K |
| Standard (4-8 cores, 8-16GB RAM) | 50K - 200K | 16 - 64 | 1K - 10K |

### **Memory Usage**

- **Ring Buffer**: `capacity * sizeof(LogEvent)` (~200-500 bytes per event)
- **Channel Buffer**: `channel_capacity * sizeof(Message)`
- **Overhead**: Minimal - only essential fields stored

### **Latency**

- **Logging Path**: <1μs for single event operations
- **Snapshot Creation**: CPU burst during serialization/compression
- **Backpressure**: Events dropped silently to prevent blocking

## 🛠️ Development

### **Building**

```bash
# Build all crates
cargo build --workspace

# Build specific crate
cargo build -p ttlog
cargo build -p ttlog-view
```

### **Testing**

```bash
# Run all tests
cargo test --workspace

# Run specific crate tests
cargo test -p ttlog
cargo test -p ttlog-view
```

### **Benchmarking**

```bash
# Run comprehensive benchmarks
cargo bench -p ttlog

# Run specific benchmark
cargo run --bin distributed_bench
cargo run --bin heavy_stress_test
```

### **Code Quality**

```bash
# Format code
cargo fmt --all

# Lint code
cargo clippy --workspace -- -D warnings

# Check for security vulnerabilities
cargo audit
```

## 📚 Examples

### **Basic Logging** (`examples/ttlog-simple/`)

```rust
// Simple logging with automatic snapshot creation
use ttlog::trace::Trace;
use tracing::info;

fn main() {
    let trace = Trace::init(10_000, 1_000);
    ttlog::panic_hook::PanicHook::install(trace.get_sender());
    
    info!("Application started");
    
    // Your application logic here
    for i in 0..100 {
        info!("Processing item {}", i);
    }
}
```

### **Async Server** (`examples/ttlog-server/`)

```rust
use tokio;
use ttlog::trace::Trace;
use tracing::info;

#[tokio::main]
async fn main() {
    let trace = Trace::init(100_000, 10_000);
    ttlog::panic_hook::PanicHook::install(trace.get_sender());
    
    info!("Server starting");
    
    // Async server logic
    tokio::spawn(async {
        for i in 0..1000 {
            info!("Handling request {}", i);
            tokio::time::sleep(tokio::time::Duration::from_millis(10)).await;
        }
    }).await.unwrap();
}
```

### **Distributed System** (`examples/ttlog-complex/`)

```rust
use tokio;
use ttlog::trace::Trace;
use tracing::info;

#[tokio::main]
async fn main() {
    let trace = Trace::init(1_000_000, 100_000);
    ttlog::panic_hook::PanicHook::install(trace.get_sender());
    
    // Simulate distributed system with multiple nodes
    let mut handles = Vec::new();
    
    for node_id in 0..8 {
        let handle = tokio::spawn(async move {
            for i in 0..10000 {
                info!(target: "node", "Node {} processing event {}", node_id, i);
                tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
            }
        });
        handles.push(handle);
    }
    
    for handle in handles {
        handle.await.unwrap();
    }
}
```

## 🔍 Viewing Snapshots

### **Using ttlog-view**

```bash
# Install viewer
cargo install --path ttlog-view

# View snapshots
ttlog-view /tmp/ttlog-*.bin

# Interactive mode
ttlog-view --interactive /tmp/ttlog-*.bin
```

### **Programmatic Access**

```rust
use ttlog::snapshot::Snapshot;
use std::fs;

fn read_snapshot(path: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
    let compressed = fs::read(path)?;
    let cbor_data = lz4::block::decompress(&compressed, None)?;
    let snapshot: Snapshot = serde_cbor::from_slice(&cbor_data)?;
    Ok(snapshot)
}

fn main() {
    let snapshot = read_snapshot("/tmp/ttlog-1234-20240814123045-panic.bin")?;
    
    println!("Service: {}", snapshot.service);
    println!("Hostname: {}", snapshot.hostname);
    println!("Events: {}", snapshot.events.len());
    
    for event in snapshot.events {
        println!("{}: {} - {}", event.timestamp_nanos, event.level, event.message);
    }
}
```

## 🎯 Use Cases

### **Microservices**
- **High-throughput logging** for API endpoints
- **Distributed tracing** across service boundaries
- **Crash recovery** for debugging production issues

### **Real-time Systems**
- **Low-latency logging** for trading systems
- **Event streaming** for IoT applications
- **Performance monitoring** for gaming servers

### **Data Processing**
- **Batch job logging** for ETL pipelines
- **Stream processing** for real-time analytics
- **Debugging** for complex data transformations

### **DevOps & Monitoring**
- **Application monitoring** with structured logs
- **Incident response** with automatic crash snapshots
- **Performance analysis** with detailed metrics

## 🔧 Advanced Features

### **Custom Event Builders**

```rust
use ttlog::event::{EventBuilder, LogLevel, FieldValue};

let event = EventBuilder::new_with_capacity(8)
    .timestamp_nanos(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap()
        .as_nanos() as u64)
    .level(LogLevel::Error)
    .target("database")
    .message("Connection failed")
    .field("connection_id", FieldValue::U64(12345))
    .field("error_code", FieldValue::U32(1001))
    .field("retry_count", FieldValue::U8(3))
    .field("timeout_ms", FieldValue::U64(5000))
    .build();
```

### **Lock-Free Buffer Operations**

```rust
use ttlog::lf_buffer::LockFreeRingBuffer;
use std::sync::Arc;

let buffer = LockFreeRingBuffer::<LogEvent>::new_shared(10000);
let buffer_clone = Arc::clone(&buffer);

// Thread-safe operations
std::thread::spawn(move || {
    for i in 0..1000 {
        let event = create_log_event(i);
        buffer_clone.push_overwrite(event);
    }
});

// Take snapshot without blocking
let events = buffer.take_snapshot();
println!("Captured {} events", events.len());
```

## 🚨 Error Handling

### **Graceful Degradation**

```rust
// TTLog handles backpressure gracefully
for i in 0..1_000_000 {
    info!("High-volume logging {}", i);
    // If buffer is full, events are dropped silently
    // Application continues without blocking
}
```

### **Snapshot Error Recovery**

```rust
// Snapshot creation errors are logged but don't crash the application
trace.request_snapshot("manual_checkpoint");
// If disk is full or permissions are wrong, error is logged to stderr
// Application continues normally
```

## 📈 Monitoring & Metrics

### **Performance Metrics**

```rust
// Monitor buffer utilization
let buffer_utilization = buffer.len() as f64 / buffer.capacity() as f64;
if buffer_utilization > 0.8 {
    warn!("Buffer utilization high: {:.1}%", buffer_utilization * 100.0);
}

// Monitor snapshot frequency
let snapshot_count = std::fs::read_dir("/tmp")
    .unwrap()
    .filter(|entry| entry.as_ref().unwrap().file_name().to_str().unwrap().starts_with("ttlog-"))
    .count();
info!("Total snapshots created: {}", snapshot_count);
```

## 🧪 Benchmark Suite

TTLog ships a dedicated benchmark crate: `ttlog-benches`.

### Run benchmarks

```bash
# Full Criterion suite (bench profile)
make bench

# Generate a comprehensive report (writes to benchmark_reports/)
make benchmark-report

# Stress/performance binaries (release)
make bench-stress
make perf-test

# Or run individually
cd ttlog-benches
cargo bench --bench distributed_bench
cargo run --release --bin max_performance all
cargo run --release --bin heavy_stress_test all
cargo run --release --bin distributed_simulator all
```

### Memory Matrix (enhanced memory benchmark)

The `max_performance` suite now includes a Memory Matrix that sweeps event counts, fields per event, and message sizes, and reports:
- Approx total bytes and bytes/event
- RSS delta in MiB (with `--features sysinfo`)
- Allocated bytes delta from jemalloc (with `--features jemalloc`)

Usage:

```bash
# Approximate memory metrics only
cargo run -p ttlog-benches --release --bin max_performance -- memory

# With RSS + jemalloc allocator stats (if available)
cargo run -p ttlog-benches --features "sysinfo jemalloc" --release --bin max_performance -- memory
```

Notes:
- Benchmarks use the bench profile (`cargo bench`) and the binaries use release (`cargo run --release`) for stable numbers.
- If Criterion warns about not completing samples in 10s, either increase target time (env vars) or reduce samples:
  - `CRITERION_SAMPLE_SIZE=10 CRITERION_MEASUREMENT_TIME=30000 make bench`

## 📊 Comprehensive Performance Results

### **Latest Benchmark Results** (max_performance suite)

```
🔬 TTLog Maximum Performance Benchmark (Unified Output)
==============================================

📊 Throughput Test Results:
+--------------------------------------+------------------------------+-------------------+------------+----------+----------------------------+------------------------+
| Test Name                            | Metric                       | Value             | Unit       | Duration | Config                     | Notes                  |
+--------------------------------------+------------------------------+-------------------+------------+----------+----------------------------+------------------------+
| Maximum Events per Second            | Events per Second            | 7,627,434         | events/sec | 5.007s   | threads=16, buffer=1000000 | Total events: 38,188,606 |
| Maximum Buffer Operations per Second | Buffer Operations per Second | 5,489,491         | ops/sec    | 5.043s   | threads=8, buffer=1000000  | Total ops: 27,683,700    |
+--------------------------------------+------------------------------+-------------------+------------+----------+----------------------------+------------------------+

📊 Concurrency Test Results:
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+
| Test Name                  | Metric          | Value  | Unit    | Duration | Config                    | Notes                                    |
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+
| Maximum Concurrent Threads | Maximum Threads | 1,024  | threads | 3.002s   | max_ops_per_sec=899787024 | Successfully ran 1024 concurrent threads |
| Maximum Concurrent Buffers | Maximum Buffers | 100,000| buffers | 12.053s  | ops_per_buffer=100        | Total operations: 10,000,000             |
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+

📊 Memory Efficiency Results:
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+
| Test Name                 | Metric                 | Value              | Unit        | Duration | Config           | Notes                                                      |
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+
| Memory Allocation Rate    | Allocations per Second | 3,240,792          | allocs/sec  | 5.000s   | events=16203961  | Est. memory: 3.50 GB                                       |
| Bytes per Event           | Memory Efficiency      | 136                | bytes/event | 0.017s   | events=41000     | Total calculated memory: 5.32 MB (includes field overhead) |
| Memory Throughput         | Memory Processing Rate | 1,134,610,185      | bytes/sec   | 5.004s   | threads=8        | Total: 5.29 GB                                             |
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+

📊 Buffer Operations (Producer/Consumer Ratios):
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
| Test Name     | Metric        | Value              | Unit        | Duration | Config                                   | Notes                                                      |
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
| Buffer 1P/1C  | Ops per Second| 5,411,406          | ops/sec     | 5.000s   | producers=1, consumers=1, buffer=1000000 | total_ops=27,058,311, Balanced                             |
| Buffer 2P/2C  | Ops per Second| 9,910,784          | ops/sec     | 5.000s   | producers=2, consumers=2, buffer=1000000 | total_ops=49,556,902, Balanced                             |
| Buffer 4P/4C  | Ops per Second| 16,757,007         | ops/sec     | 5.000s   | producers=4, consumers=4, buffer=1000000 | total_ops=83,792,877, Balanced                             |
| Buffer 8P/8C  | Ops per Second| 15,423,638         | ops/sec     | 5.001s   | producers=8, consumers=8, buffer=1000000 | total_ops=77,133,587, Balanced                             |
| Buffer 8P/4C  | Ops per Second| 16,616,877         | ops/sec     | 5.001s   | producers=8, consumers=4, buffer=1000000 | total_ops=83,095,228, Producer heavy                       |
| Buffer 4P/8C  | Ops per Second| 12,347,540         | ops/sec     | 5.001s   | producers=4, consumers=8, buffer=1000000 | total_ops=61,746,034, Consumer heavy                       |
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
```

### **Key Performance Highlights**

- **🚀 Peak Throughput**: 7.6M events/second with 16 threads
- **⚡ Buffer Operations**: Up to 16.7M operations/second (4P/4C configuration)
- **🔄 Massive Concurrency**: Successfully handles 1,024 concurrent threads
- **💾 Buffer Scalability**: Supports 100,000 concurrent buffers
- **🧠 Memory Efficiency**: Only 136 bytes per event (including field overhead)
- **📈 Memory Throughput**: 1.1 GB/second sustained processing rate

To view full details (all groups and inputs), open:
- `benchmark_reports/comprehensive_report.txt`
- Criterion HTML reports under `target/criterion/`

---

## 🔮 Future Enhancements

### **Planned Features**
- **Remote Storage**: Upload snapshots to cloud storage (S3, GCS, Azure)
- **Log Rotation**: Automatic cleanup of old snapshot files
- **Metrics Export**: Prometheus metrics for monitoring
- **Query Language**: SQL-like queries for log analysis
- **Real-time Streaming**: Live log streaming to external systems

### **Performance Improvements**
- **Zero-copy Serialization**: Reduce memory allocations
- **SIMD Compression**: Faster LZ4 compression with SIMD
- **Memory Mapping**: Direct memory mapping for large buffers
- **NUMA Awareness**: Optimize for NUMA architectures

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### **Development Setup**

```bash
# Clone the repository
git clone https://github.com/wildduck/ttlog.git
cd ttlog

# Install development tools
make install-tools

# Run development checks
make all

# Run tests
make test

# Run benchmarks
make bench
```

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **Crossbeam**: For the excellent lock-free data structures
- **Tracing**: For the robust tracing ecosystem
- **Serde**: For the flexible serialization framework
- **LZ4**: For the fast compression algorithm

---

**🚀 TTLog: High-performance logging for the modern distributed world!**

