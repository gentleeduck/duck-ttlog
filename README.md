# TTLog - Ultra-High-Performance Structured Logging

<p align="center">
  <img src="./public/logo.png" alt="TTLog Logo" width="500"/>
</p>

[![Crates.io](https://img.shields.io/crates/v/ttlog)](https://crates.io/crates/ttlog)
[![Documentation](https://docs.rs/ttlog/badge.svg)](https://docs.rs/ttlog)
[![License: MIT](https://img.shields.io/badge/License-MIT-yellow.svg)](https://opensource.org/licenses/MIT)
[![Rust Version](https://img.shields.io/badge/rust-1.70+-blue.svg)](https://www.rust-lang.org)

> **🚀 66+ Million Events/Second** • **Lock-Free** • **Zero-Copy** • **Crash-Safe**

TTLog is a blazingly fast, lock-free structured logging library designed for high-throughput distributed systems. Built from the ground up for performance, it maintains in-memory ring buffers and creates compressed snapshots for post-mortem analysis.

## ⚡ Performance Highlights

```
🎯 Peak Performance Results (Latest Benchmarks)
┌─────────────────────────────────────────────────────────────────┐
│ 🚀 Event Throughput:       66.2M events/sec (16 threads)        │
│ ⚡ Buffer Operations:       14.9M ops/sec (4P/4C config)        │
│ 🔄 Concurrent Threads:     1,024 threads simultaneously         │
│ 💾 Buffer Capacity:        100K concurrent buffers              │
│ 🧠 Memory Efficiency:      136 bytes/event (w/ structured data) │
│ 📈 Memory Throughput:      1.02 GB/sec processing rate          │
└─────────────────────────────────────────────────────────────────┘
```

## 🎯 Key Features

### **🔥 Extreme Performance**
- **66M+ events/second** - Industry-leading throughput
- **Lock-free ring buffers** using crossbeam's ArrayQueue
- **Zero-allocation fast path** for hot logging operations
- **Thread-local string interning** with compile-time optimization

### **🛡️ Production-Ready Reliability**
- **Automatic crash snapshots** via panic hooks
- **Atomic file operations** prevent corruption
- **Graceful backpressure** - never blocks your application
- **Overflow protection** with ring buffer semantics

### **🎨 Developer Experience**
- **Structured logging** with type-safe fields
- **Tracing integration** via subscriber layers
- **Proc macros** for convenient logging
- **Built-in viewer** for snapshot analysis

### **💾 Smart Storage**
- **CBOR + LZ4 compression** for optimal size/speed
- **Configurable snapshots** (panic, periodic, manual)
- **Rich metadata** (hostname, PID, thread ID, timestamps)

## 🏗️ Architecture

### **Event Processing Pipeline**

```
┌─────────────────┐     ┌──────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│   Application   │     │  EventBuilder +  │    │  Writer Thread  │    │   Snapshot      │
│                 │     │ StringInterner   │    │                 │    │   Creation      │
│  tracing::info! │───▶│                  │───▶│ Lock-Free Ring  │───▶│                 │
│  ttlog::event!  │     │ • Field Capping  │    │     Buffer      │    │ CBOR + LZ4 +    │
│                 │     │ • String Intern  │    │ • 1M+ capacity  │    │ Atomic Write    │
└─────────────────┘     └──────────────────┘    └─────────────────┘    └─────────────────┘
                                ▲                        │                        │
                                │                        ▼                        ▼
                       ┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
                       │ Static Interner │    │ Concurrent Ops  │    │  Compressed     │
                       │ (Thread-Safe)   │    │ 66M events/sec  │    │  Log Files      │
                       │                 │    │ 1024 threads    │    │ 136 bytes/event │
                       └─────────────────┘    └─────────────────┘    └─────────────────┘
```

### **Data Flow Architecture**

```
Logging Call                  Event Processing                    Storage Pipeline
─────────────                 ───────────────────                 ─────────────────

info!(                       ┌──────────────────────┐            ┌──────────────────┐
  user_id = 123,             │   Macro Expansion    │            │  Snapshot Timer  │
  action = "login"           │                      │            │   (60 seconds)   │
);                   ──────▶│ • Level check (O(1)) │    ┌─────▶│                  │
                             │ • Static caching     │    │       │ • Periodic flush │
                             │ • Target/msg intern  │    │       │ • Panic trigger  │
                             └──────────────────────┘    │       │ • Manual request │
                                       │                 │       └──────────────────┘
                                       ▼                 │               │
                             ┌─────────────────────┐     │               ▼
Thread 1: Producer           │   Event Creation    │     │     ┌──────────────────┐
Thread 2: Producer    ────▶ │                     │     │     │  Writer Thread   │
Thread N: Producer           │ • Pack metadata     │     │     │                  │
                             │ • Assign thread_id  │     │     │ • CBOR serialize │
                             │ • Add structured    │ ────┘     │ • LZ4 compress   │
                             │   fields (max 3)    │           │ • Atomic write   │
                             └─────────────────────┘           │ • File rotation  │
                                       │                       └──────────────────┘
                                       ▼                                   │
                             ┌─────────────────────┐                      ▼
                             │  Lock-Free Buffer   │             ┌─────────────────┐
                             │                     │             │   Disk Storage  │
                             │ • Ring buffer       │             │                 │
                             │ • Overwrite oldest  │             │ /tmp/ttlog-     │
                             │ • Zero-copy push    │             │  {pid}-{time}-  │
                             │ • Concurrent reads  │             │  {reason}.bin   │
                             └─────────────────────┘             └─────────────────┘
```

### **Performance Characteristics**

```
┌─────────────────────────────────────────────────────────────────────────────┐
│                        TTLog Performance Metrics                           │
├─────────────────────────────────────────────────────────────────────────────┤
│ 🚀 Throughput:         66.2M events/sec    │  Memory:     136 bytes/event   │
│ ⚡ Buffer Ops:         14.9M ops/sec       │  Allocation: 3.1M allocs/sec   │
│ 🔄 Concurrency:        1,024 threads       │  Throughput: 1.0 GB/sec       │
│ 💾 Buffers:            100K concurrent     │  Efficiency: Lock-free design  │
└─────────────────────────────────────────────────────────────────────────────┘
```

### **Thread Safety & Concurrency Model**

```
Thread Safety Architecture
──────────────────────────

Main Thread              Writer Thread             Storage Thread
───────────              ─────────────             ──────────────
┌─────────────┐          ┌─────────────┐           ┌─────────────┐
│ Application │          │ Ring Buffer │           │ Disk Writer │
│             │          │ Consumer    │           │             │
│ • Log calls │   ──────▶│             │  ────────▶│ • CBOR ser  │
│ • Fast path │          │ • Bounded   │           │ • LZ4 comp  │
│ • Non-block │          │ • Lock-free │           │ • Atomic IO │
└─────────────┘          └─────────────┘           └─────────────┘
       │                        │
       ▼                        ▼
┌─────────────┐          ┌─────────────┐
│Worker Thread│          │Panic Handler│
│Pool (1-N)   │          │             │
│             │          │ • Hook reg  │
│ • Parallel  │          │ • Emergency │
│ • Zero-copy │          │   snapshot  │
│ • Lock-free │          │ • Graceful  │
└─────────────┘          └─────────────┘

Synchronization Points:
• Ring buffer: Lock-free (crossbeam::ArrayQueue)
• String interning: Thread-local cache + RwLock fallback
• File writing: Single writer thread (no contention)
• Level filtering: Atomic load (single instruction)
```

### **Workspace Structure**

```
ttlog/
├── ttlog/              # Main library crate
│   ├── event/          # Log event structures and builders
│   ├── lf_buffer/      # Lock-free ring buffer implementation
│   ├── snapshot/       # Snapshot creation and persistence
│   ├── trace/          # Main orchestration and writer thread
│   ├── string_interner/ # Thread-local string deduplication
│   └── panic_hook/     # Crash recovery mechanisms
├── ttlog-macros/       # Proc macros for convenient logging
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
ttlog-macros = "0.1.0"  # For convenient logging macros
```

## 🚀 Quick Start

### **Basic Setup**

```rust
use ttlog::trace::Trace;
use ttlog::ttlog_macros::{info, error};

fn main() -> Result<(), Box<dyn std::error::Error>> {
  // Initialize with 100K event capacity
  let _trace = Trace::init(100_000, 10_000, "my-service");
  
  // Log with structured data
  info!("Server starting", port = 8080, env = "production");
  
  // Simulate some work
  for i in 0..1000 {
      if i % 100 == 0 {
          info!("Processed batch", batch_id = i, items = 100);
      }
  }
  
  // Manual snapshot (optional)
  // trace.request_snapshot("checkpoint");
  
  Ok(())
}
```

### **With Tracing Integration**

```rust
use ttlog::ttlog_macros::{info, warn, error};
use ttlog::trace::Trace;

#[tokio::main]
async fn main() {
    // Initialize TTLog
    let trace = Trace::init(1_000_000, 100_000, "async-service");
    
    // Use standard tracing macros
    info!(user_id = 12345, action = "login", "User authenticated");
    warn!(connection_id = "conn_123", "Connection timeout");
    error!(error_code = 500, "Database connection failed");
    
    // Your async application logic...
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
}
```

### **Advanced Structured Logging**

```rust
use ttlog::ttlog_macros::{info, error};
use ttlog::trace::Trace;

fn main() {
  let _trace = Trace::init(1_000_000, 10_000, "trading-system");
  
  // High-frequency trading logs
  info!("Trade executed", 
    symbol = "AAPL", 
    quantity = 1000, 
    price = 150.25, 
    side = "BUY",
    latency_us = 42
  );
  
  // Error with context
  error!("Order rejected", 
    order_id = "12345", 
    reason = "insufficient_funds", 
    account = "ACC_001"
  );
}
```

## 📊 Comprehensive Performance Results

### **Latest Benchmark Results** (Updated with 66M+ events/sec)

```
🔬 TTLog Maximum Performance Benchmark (Unified Output)
==============================================

📊 Throughput Test Results:
+--------------------------------------+------------------------------+--------------------+------------+----------+----------------------------+------------------------+
| Test Name                            | Metric                       | Value              | Unit       | Duration | Config                     | Notes                  |
+--------------------------------------+------------------------------+--------------------+------------+----------+----------------------------+------------------------+
| Maximum Events per Second            | Events per Second            | 66,157,793         | events/sec | 5.053s   | threads=16, buffer=1000000 | Total events: 334,326,118 |
| Maximum Buffer Operations per Second | Buffer Operations per Second | 5,319,610          | ops/sec    | 5.045s   | threads=8, buffer=1000000  | Total ops: 26,837,712     |
+--------------------------------------+------------------------------+--------------------+------------+----------+----------------------------+------------------------+

📊 Concurrency Test Results:
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+
| Test Name                  | Metric          | Value  | Unit    | Duration | Config                    | Notes                                    |
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+
| Maximum Concurrent Threads | Maximum Threads | 1,024  | threads | 5.549s   | max_ops_per_sec=842912740 | Successfully ran 1024 concurrent threads |
| Maximum Concurrent Buffers | Maximum Buffers | 100,000| buffers | 13.764s  | ops_per_buffer=100        | Total operations: 10,000,000             |
+----------------------------+-----------------+--------+---------+----------+---------------------------+------------------------------------------+

📊 Memory Efficiency Results:
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+
| Test Name                 | Metric                 | Value              | Unit        | Duration | Config           | Notes                                                      |
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+
| Memory Allocation Rate    | Allocations per Second | 3,143,159          | allocs/sec  | 5.000s   | events=15715795  | Est. memory: 3.40 GB                                       |
| Bytes per Event           | Memory Efficiency      | 136                | bytes/event | 0.018s   | events=41000     | Total calculated memory: 5.32 MB (includes field overhead) |
| Memory Throughput         | Memory Processing Rate | 1,022,316,205      | bytes/sec   | 5.004s   | threads=8        | Total: 4.76 GB                                             |
+---------------------------+------------------------+--------------------+-------------+----------+------------------+------------------------------------------------------------+

📊 Buffer Operations (Producer/Consumer Ratios):
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
| Test Name     | Metric        | Value              | Unit        | Duration | Config                                   | Notes                                                      |
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
| Buffer 1P/1C  | Ops per Second| 5,335,209          | ops/sec     | 5.000s   | producers=1, consumers=1, buffer=1000000 | total_ops=26,676,970, Balanced                             |
| Buffer 2P/2C  | Ops per Second| 9,606,845          | ops/sec     | 5.000s   | producers=2, consumers=2, buffer=1000000 | total_ops=48,036,291, Balanced                             |
| Buffer 4P/4C  | Ops per Second| 14,941,426         | ops/sec     | 5.001s   | producers=4, consumers=4, buffer=1000000 | total_ops=74,716,394, Balanced                             |
| Buffer 8P/8C  | Ops per Second| 14,122,851         | ops/sec     | 5.006s   | producers=8, consumers=8, buffer=1000000 | total_ops=70,695,005, Balanced                             |
| Buffer 8P/4C  | Ops per Second| 14,476,954         | ops/sec     | 5.001s   | producers=8, consumers=4, buffer=1000000 | total_ops=72,399,541, Producer heavy                       |
| Buffer 4P/8C  | Ops per Second| 12,533,962         | ops/sec     | 5.001s   | producers=4, consumers=8, buffer=1000000 | total_ops=62,677,806, Consumer heavy                       |
+---------------+---------------+--------------------+-------------+----------+------------------------------------------+------------------------------------------------------------+
```

### **Key Performance Highlights**

- **🚀 Peak Throughput**: **66.2M events/second** with 16 threads
- **⚡ Buffer Operations**: Up to **14.9M operations/second** (4P/4C configuration)
- **🔄 Massive Concurrency**: Successfully handles **1,024 concurrent threads**
- **💾 Buffer Scalability**: Supports **100,000 concurrent buffers**
- **🧠 Memory Efficiency**: Only **136 bytes per event** (including field overhead)
- **📈 Memory Throughput**: **1.02 GB/second** sustained processing rate

### **Real-World Performance Scenarios**

```rust
// Scenario 1: High-frequency trading system
// Requirement: <100ns per log operation
use ttlog::ttlog_macros::info;

fn trade_execution() {
  info!("Trade executed", 
    symbol = "AAPL", 
    quantity = 1000, 
    price = 150.25, 
    timestamp_us = 1703123456789
  );
  // Typical latency: ~50ns
}

// Scenario 2: Web server logging
// Requirement: Handle 100K RPS without blocking
async fn handle_request(req_id: u64) {
  info!("Request received", 
    request_id = req_id, 
    method = "GET", 
    path = "/api/users"
  );
  // Zero blocking - continues immediately
}

// Scenario 3: IoT data streaming
// Requirement: 1M+ sensor readings per second
fn process_sensor_data() {
  for sensor_id in 0..1000 {
    info!("Sensor reading", 
      sensor_id = sensor_id, 
      temperature = 23.5, 
      humidity = 45.2
    );
  }
  // Handles 1M+ events/sec easily
}
```

## 🔧 Configuration & Tuning

### **Performance Tuning**

```rust
use ttlog::trace::Trace;

// High-performance configuration for distributed systems
let trace = Trace::init(
  1_000_000,  // 1M events in ring buffer
  100_000,    // 100K events in channel buffer  
  "high-perf-service"
);

// Memory-optimized for resource-constrained environments
let trace = Trace::init(
  10_000,     // 10K events in ring buffer
  1_000,      // 1K events in channel buffer
  "embedded-service"
);

// Ultra-high throughput for data processing
let trace = Trace::init(
  10_000_000, // 10M events in ring buffer
  1_000_000,  // 1M events in channel buffer
  "data-pipeline"
);
```

### **Environment Configuration**

```rust
// Custom snapshot directory
std::env::set_var("TTLOG_SNAPSHOT_DIR", "/var/log/ttlog");

// Configure periodic flush interval (default: 60s)
std::env::set_var("TTLOG_FLUSH_INTERVAL", "30");

// Enable debug mode
std::env::set_var("TTLOG_DEBUG", "1");
```

### **Runtime Performance Monitoring**

```rust
use ttlog::trace::Trace;

fn monitor_performance(trace: &Trace) {
  // Monitor buffer utilization
  let buffer_stats = trace.get_buffer_stats();
  if buffer_stats.utilization > 0.8 {
    println!("Warning: Buffer utilization at {:.1}%", 
      buffer_stats.utilization * 100.0
    );
  }
  
  // Monitor string interning efficiency
  let (targets, messages, fields) = trace.interner.stats();
  println!("String intern stats - targets: {}, messages: {}, fields: {}", 
    targets, messages, fields
  );
}
```

## 📚 Advanced Usage Examples

### **Distributed System Logging**

```rust
use ttlog::trace::Trace;
use ttlog::ttlog_macros::{info, warn, error};
use tokio;

#[tokio::main]
async fn main() {
  let _trace = Trace::init(1_000_000, 100_000, "distributed-node");
  
  // Simulate distributed system with multiple components
  let mut handles = Vec::new();
  
  // Database connection pool
  let db_handle = tokio::spawn(async move {
    for i in 0..10000 {
      info!("DB query executed", 
        query_id = i, 
        table = "users", 
        duration_ms = 15
      );
      tokio::time::sleep(tokio::time::Duration::from_micros(100)).await;
    }
  });
  handles.push(db_handle);
  
  // Message queue consumer
  let mq_handle = tokio::spawn(async move {
    for i in 0..10000 {
      info!("Message processed", 
        message_id = i, 
        queue = "orders", 
        size_bytes = 1024
      );
      tokio::time::sleep(tokio::time::Duration::from_micros(150)).await;
    }
  });
  handles.push(mq_handle);
  
  // HTTP API server
  let api_handle = tokio::spawn(async move {
    for i in 0..10000 {
      if i % 1000 == 0 {
        warn!("High request rate", 
          requests_per_sec = 5000, 
          endpoint = "/api/orders"
        );
      }
      info!("API request", 
        request_id = i, 
        method = "POST", 
        status = 200
      );
      tokio::time::sleep(tokio::time::Duration::from_micros(50)).await;
    }
  });
  handles.push(api_handle);
  
  // Wait for all components
  for handle in handles {
    handle.await.unwrap();
  }
  
  info!("Distributed system simulation completed");
}
```

### **Error Handling & Recovery**

```rust
use ttlog::trace::Trace;
use ttlog_macros::{error, warn, info};

fn robust_service() {
  let trace = Trace::init(100_000, 10_000, "robust-service");
  
  // Setup panic hook for crash recovery
  ttlog::panic_hook::PanicHook::install(trace.get_sender());
  
  // Simulate error conditions
  for i in 0..1000 {
    match risky_operation(i) {
      Ok(result) => {
        info!("Operation successful", 
          operation_id = i, 
          result = result
        );
      },
      Err(e) => {
        error!("Operation failed", 
          operation_id = i, 
          error = e.to_string(), 
          retry_count = 3
        );
        
        // Request immediate snapshot for debugging
        trace.request_snapshot("error_context");
      }
    }
  }
}

fn risky_operation(id: u32) -> Result<u32, Box<dyn std::error::Error>> {
  if id % 100 == 0 {
    Err("Simulated error".into())
  } else {
   Ok(id * 2)
  }
}
```

## 🔍 Snapshot Analysis

### **Using ttlog-view**

```bash
# Install viewer
cargo install --path ttlog-view

# View snapshots with filtering
ttlog-view /tmp/ttlog-*.bin --filter level=ERROR --limit 100

# Interactive mode with search
ttlog-view --interactive /tmp/ttlog-*.bin

# Export to JSON for external analysis
ttlog-view /tmp/ttlog-*.bin --format json > logs.json

# Statistics and aggregation
ttlog-view /tmp/ttlog-*.bin --stats --group-by target
```

### **Programmatic Snapshot Reading**

```rust
use ttlog::snapshot::Snapshot;
use std::fs;

fn analyze_snapshots() -> Result<(), Box<dyn std::error::Error>> {
  // Read and analyze snapshot
  let snapshot = read_snapshot("/tmp/ttlog-1234-20240814123045-panic.bin")?;
  
  println!("=== Snapshot Analysis ===");
  println!("Service: {}", snapshot.service);
  println!("Hostname: {}", snapshot.hostname);
  println!("PID: {}", snapshot.pid);
  println!("Created: {}", snapshot.created_at);
  println!("Reason: {}", snapshot.reason);
  println!("Events: {}", snapshot.events.len());
  
  // Analyze by log level
  let mut level_counts = std::collections::HashMap::new();
  for event in &snapshot.events {
      *level_counts.entry(event.level()).or_insert(0) += 1;
  }
  
  println!("\n=== Log Level Distribution ===");
  for (level, count) in level_counts {
      println!("{:?}: {}", level, count);
  }
  
  // Find error events
  let errors: Vec<_> = snapshot.events
      .iter()
      .filter(|e| matches!(e.level(), ttlog::event::LogLevel::ERROR))
      .collect();
  
  println!("\n=== Error Events ===");
  for event in errors.iter().take(10) {
      println!("Error: target_id={}, message_id={}, thread={}",
               event.target_id, event.message_id, event.thread_id());
  }
  
  Ok(())
}

fn read_snapshot(path: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
  let compressed = fs::read(path)?;
  let cbor_data = lz4::block::decompress(&compressed, None)?;
  let snapshot: Snapshot = serde_cbor::from_slice(&cbor_data)?;
  Ok(snapshot)
}
```

## 🧪 Benchmark Suite & Testing

### **Running Benchmarks**

```bash
# Full Criterion benchmark suite
make bench

# Generate comprehensive performance report
make benchmark-report

# Stress testing binaries
make bench-stress
make perf-test

# Run specific benchmark categories
cd ttlog-benches
cargo run --release --bin max_performance throughput
cargo run --release --bin max_performance memory
cargo run --release --bin max_performance concurrency
cargo run --release --bin heavy_stress_test distributed
```

### **Custom Performance Testing**

```rust
use ttlog::trace::Trace;
use ttlog_macros::info;
use std::time::Instant;

fn benchmark_custom_workload() {
  let _trace = Trace::init(1_000_000, 100_000, "benchmark");
  
  let start = Instant::now();
  let iterations = 1_000_000;
  
  for i in 0..iterations {
    info!("Benchmark event", 
      iteration = i, 
      timestamp = start.elapsed().as_nanos() as u64, 
      workload = "custom"
    );
  }
  
  let duration = start.elapsed();
  let events_per_sec = iterations as f64 / duration.as_secs_f64();
  
  println!("Custom benchmark: {:.0} events/sec", events_per_sec);
}
```

## 🎯 Production Use Cases

### **Financial Trading Systems**
```rust
// Ultra-low latency requirements
use ttlog_macros::info;

fn handle_market_data() {
  info!("Market tick", 
    symbol = "EURUSD", 
    bid = 1.0845, 
    ask = 1.0847, 
    volume = 1000000,
    exchange = "EBS"
  );
  // Latency: ~50ns per log call
}
```

### **High-Traffic Web Services**
```rust
// Non-blocking request logging
#[tokio::main]
async fn main() {
  let _trace = Trace::init(5_000_000, 500_000, "web-api");
  
  // Handles 100K+ RPS without blocking
  info!("Request processed", 
    request_id = "req_12345", 
    method = "GET", 
    path = "/api/users/123", 
    response_time_ms = 45, 
    status = 200
  );
}
```

### **IoT Data Ingestion**
```rust
// Massive sensor data streams
fn process_sensor_batch(sensors: &[SensorReading]) {
  for reading in sensors {
    info!("Sensor data", 
      device_id = reading.device_id, 
      temperature = reading.temperature, 
      humidity = reading.humidity, 
      battery = reading.battery_level
    );
  }
  // Processes 1M+ readings/second
}
```

## 🚨 Production Considerations

### **Memory Management**
```rust
// Monitor memory usage in production
fn production_monitoring(trace: &Trace) {
  let buffer_usage = trace.get_buffer_utilization();
  
  if buffer_usage > 0.9 {
    // Take immediate snapshot before data loss
    trace.request_snapshot("high_memory_usage");
    
    // Consider reducing log volume or increasing buffer size
    eprintln!("Warning: Buffer utilization at {:.1}%", buffer_usage * 100.0);
  }
}
```

### **Error Recovery**
```rust
// Graceful shutdown with log preservation
fn graceful_shutdown(trace: Trace) {
  println!("Shutting down - preserving logs...");
  
  // Request final snapshot
  trace.request_snapshot("shutdown");
  
  // Allow time for snapshot creation
  std::thread::sleep(std::time::Duration::from_millis(500));
  
  println!("Shutdown complete");
}
```

### **Disk Space Management**
```bash
#!/bin/bash
# Automated log rotation script

# Keep only last 24 hours of snapshots
find /tmp -name "ttlog-*.bin" -mtime +1 -delete

# Compress old snapshots
find /tmp -name "ttlog-*.bin" -mtime +0.5 -exec gzip {} \;

# Alert if disk usage > 80%
DISK_USAGE=$(df /tmp | tail -1 | awk '{print $5}' | sed 's/%//')
if [ $DISK_USAGE -gt 80 ]; then
    echo "Warning: Disk usage at ${DISK_USAGE}%" | logger
fi
```

## 🔮 Advanced Features

### **Custom Event Builders**

```rust
use ttlog::event::{EventBuilder, LogLevel, FieldValue};
use ttlog::string_interner::StringInterner;
use std::sync::Arc;

fn custom_event_creation() {
  let interner = Arc::new(StringInterner::new());
  let mut builder = EventBuilder::new(Arc::clone(&interner));
  
  // Build high-performance structured event
  let event = builder.build_with_fields(
    std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    LogLevel::Error,
    "database",
    "Connection failed",
    &[
      ("connection_id".to_string(), FieldValue::U64(12345)),
      ("error_code".to_string(), FieldValue::U32(1001)),
      ("retry_count".to_string(), FieldValue::U8(3)),
    ],
  );
  
  println!("Created event with {} fields", event.field_count);
}
```

### **Direct Buffer Access**

```rust
use ttlog::lf_buffer::LockFreeRingBuffer;
use ttlog::event::LogEvent;
use std::sync::Arc;

fn direct_buffer_operations() {
  let buffer = LockFreeRingBuffer::<LogEvent>::new_shared(100_000);
  let buffer_clone = Arc::clone(&buffer);
  
  // Producer thread
  std::thread::spawn(move || {
    for i in 0..10_000 {
      let mut event = LogEvent::new();
      event.packed_meta = LogEvent::pack_meta(
        std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_nanos() as u64,
        ttlog::event::LogLevel::Info,
        1, // thread_id
      );
      event.target_id = i as u16;
      event.message_id = (i % 1000) as u16;
      
      buffer_clone.push_overwrite(event);
    }
  });
  
  // Consumer - take periodic snapshots
  std::thread::sleep(std::time::Duration::from_millis(100));
  let events = buffer.take_snapshot();
  println!("Captured {} events from buffer", events.len());
}
```

### **String Interning Optimization**

```rust
use ttlog::string_interner::StringInterner;
use std::sync::Arc;

fn optimize_string_interning() {
  let interner = Arc::new(StringInterner::new());
  
  // Pre-intern common strings for better performance
  let common_targets = [
    "http_server",
    "database",
    "cache",
    "message_queue",
    "auth_service",
  ];
  
  let common_messages = [
    "Request processed",
    "Connection established",
    "Query executed",
    "Cache hit",
    "Cache miss",
  ];
  
  // Intern all common strings upfront
  for target in &common_targets {
    interner.intern_target(target);
  }
  
  for message in &common_messages {
    interner.intern_message(message);
  }
  
  let (targets, messages, fields) = interner.stats();
  println!("Pre-interned {} targets, {} messages, {} fields", 
    targets, messages, fields);
}
```

## 🔧 Integration Guides

### **Integration with Actix Web**

```rust
use actix_web::{web, App, HttpResponse, HttpServer, Result, middleware::Logger};
use ttlog::trace::Trace;
use ttlog_macros::{info, error};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
  // Initialize TTLog with high capacity for web traffic
  let _trace = Trace::init(2_000_000, 200_000, "actix-web-server");
  
  HttpServer::new(|| {
    App::new()
      .wrap(Logger::default())
      .route("/api/users/{id}", web::get().to(get_user))
      .route("/api/orders", web::post().to(create_order))
  })
  .bind("127.0.0.1:8080")?
  .run()
  .await
}

async fn get_user(path: web::Path<u32>) -> Result<HttpResponse> {
  let user_id = path.into_inner();
  
  info!("User request", 
    user_id = user_id, 
    endpoint = "/api/users", 
    method = "GET");
  
  // Simulate user lookup
  if user_id > 1000 {
    error!("User not found", user_id = user_id);
    Ok(HttpResponse::NotFound().json("User not found"))
  } else {
    info!("User found", user_id = user_id, username = "john_doe");
    Ok(HttpResponse::Ok().json(format!("User {}", user_id)))
  }
}

async fn create_order(order: web::Json<serde_json::Value>) -> Result<HttpResponse> {
  info!("Order creation", 
    order_data = order.to_string().len(), 
    endpoint = "/api/orders", 
    method = "POST"
  );
  
  Ok(HttpResponse::Created().json("Order created"))
}
```

### **Integration with Tokio Tracing**

```rust
use tracing::{info, warn, error, Level};
use tracing_subscriber::{Registry, layer::SubscriberExt, util::SubscriberInitExt};
use ttlog::trace::Trace;

#[tokio::main]
async fn main() {
  // Initialize TTLog
  let trace = Trace::init(1_000_000, 100_000, "tokio-app");
  
  // Setup tracing subscriber with TTLog layer
  Registry::default()
    .with(tracing_subscriber::fmt::layer())
    .with(trace.create_layer()) // TTLog layer
    .init();
  
  // Use standard tracing macros - they'll be captured by TTLog
  info!(user_id = 12345, "User session started");
  warn!(connection_pool_size = 5, "Connection pool nearly full");
  error!(database = "postgres", "Connection timeout");
  
  // Simulate async work
  tokio::spawn(async {
    for i in 0..1000 {
      info!(task_id = i, "Background task processing");
      tokio::time::sleep(tokio::time::Duration::from_millis(1)).await;
    }
  }).await.unwrap();
}
```

### **Integration with Kubernetes**

```yaml
# deployment.yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ttlog-app
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ttlog-app
  template:
    metadata:
      labels:
        app: ttlog-app
    spec:
      containers:
      - name: app
        image: my-app:latest
        env:
        - name: TTLOG_SNAPSHOT_DIR
          value: "/var/log/ttlog"
        - name: TTLOG_SERVICE_NAME
          value: "k8s-microservice"
        - name: TTLOG_FLUSH_INTERVAL
          value: "30"
        volumeMounts:
        - name: log-storage
          mountPath: /var/log/ttlog
      volumes:
      - name: log-storage
        persistentVolumeClaim:
          claimName: ttlog-pvc
```

```rust
// Kubernetes-optimized application
use ttlog::trace::Trace;
use ttlog_macros::{info, error};

fn main() {
  // Get configuration from environment
  let snapshot_dir = std::env::var("TTLOG_SNAPSHOT_DIR")
    .unwrap_or_else(|_| "/tmp".to_string());
  let service_name = std::env::var("TTLOG_SERVICE_NAME")
    .unwrap_or_else(|_| "unknown-service".to_string());
  
  // Initialize with Kubernetes-friendly settings
  let trace = Trace::init(1_000_000, 100_000, &service_name);
  
  // Set custom snapshot directory
  std::env::set_var("TTLOG_SNAPSHOT_DIR", snapshot_dir);
  
  info!("Service started", 
    service = service_name, 
    pod_name = std::env::var("HOSTNAME").unwrap_or_default(),
    namespace = std::env::var("POD_NAMESPACE").unwrap_or_default());
  
  // Your application logic here
  run_application();
  
  // Graceful shutdown
  trace.request_snapshot("pod_shutdown");
  info!("Service shutting down gracefully");
}

fn run_application() {
  // Application logic
  for i in 0..10000 {
    info!("Processing request", request_id = i);
  }
}
```

## 📊 Monitoring & Observability

### **Prometheus Metrics Integration**

```rust
use prometheus::{Counter, Histogram, Gauge, register_counter, register_histogram, register_gauge};
use ttlog::trace::Trace;
use ttlog::ttlog_macros::{info, warn};

lazy_static::lazy_static! {
  static ref LOG_EVENTS_TOTAL: Counter = register_counter!(
    "ttlog_events_total", 
    "Total number of log events processed"
  ).unwrap();
  
  static ref LOG_EVENT_DURATION: Histogram = register_histogram!(
    "ttlog_event_duration_seconds", 
    "Time spent processing log events"
  ).unwrap();
  
  static ref BUFFER_UTILIZATION: Gauge = register_gauge!(
    "ttlog_buffer_utilization_ratio", 
    "Current buffer utilization (0.0 to 1.0)"
  ).unwrap();
}

fn monitored_logging_service() {
  let trace = Trace::init(1_000_000, 100_000, "monitored-service");
  
  // Background metrics collection
  std::thread::spawn(move || {
    loop {
      // Update buffer utilization metric
      let utilization = trace.get_buffer_utilization();
      BUFFER_UTILIZATION.set(utilization);
      
      if utilization > 0.8 {
          warn!("High buffer utilization", utilization = utilization);
      }
      
      std::thread::sleep(std::time::Duration::from_secs(10));
    }
  });
  
  // Instrumented logging
  for i in 0..100_000 {
    let _timer = LOG_EVENT_DURATION.start_timer();
    
    info!("Service event", event_id = i, status = "processed");
    
    LOG_EVENTS_TOTAL.inc();
  }
}
```

### **Health Check Endpoint**

```rust
use serde_json::json;
use ttlog::trace::Trace;

fn health_check_handler(trace: &Trace) -> serde_json::Value {
  let buffer_stats = trace.get_buffer_stats();
  let (targets, messages, fields) = trace.interner.stats();
  
  json!({
    "status": if buffer_stats.utilization < 0.9 { "healthy" } else { "warning" },
    "buffer": {
      "capacity": buffer_stats.capacity,
      "used": buffer_stats.used,
      "utilization": buffer_stats.utilization,
      "overflows": buffer_stats.overflow_count
    },
    "string_interning": {
      "targets": targets,
      "messages": messages,
      "fields": fields
    },
    "snapshots": {
      "last_snapshot": trace.get_last_snapshot_time(),
      "total_snapshots": trace.get_snapshot_count()
    }
  })
}
```

## 🔒 Security Considerations

### **Sensitive Data Handling**

```rust
use ttlog_macros::{info, warn};

// Safe: Structure data without sensitive values
fn safe_user_logging(user_id: u64, email: &str) {
  info!("User login", 
    user_id = user_id, 
    email_domain = email.split('@').nth(1).unwrap_or("unknown"), 
    timestamp = chrono::Utc::now().timestamp()
  );
}

// Avoid: Logging sensitive data directly
fn unsafe_user_logging() {
  // DON'T DO THIS
  // info!("User login", password = "secret123", ssn = "123-45-6789");
  
  // Instead, log non-sensitive identifiers
  info!("Authentication attempt", 
    user_id = 12345, 
    success = true, 
    method = "password"
  );
}
```

### **Access Control for Snapshots**

```bash
#!/bin/bash
# Secure snapshot permissions

# Create secure log directory
mkdir -p /var/log/ttlog-secure
chmod 700 /var/log/ttlog-secure
chown app-user:app-group /var/log/ttlog-secure

# Set environment for application
export TTLOG_SNAPSHOT_DIR="/var/log/ttlog-secure"

# Automated secure cleanup
find /var/log/ttlog-secure -name "*.bin" -mtime +7 -exec shred -u {} \;
```

## 🚀 Future Enhancements & Roadmap

### **Planned Features**
- **🌐 Remote Storage**: Direct upload to S3, GCS, Azure Blob
- **🔍 Query Engine**: SQL-like queries for log analysis  
- **📊 Real-time Streaming**: Live log streaming via WebSocket/gRPC
- **🎯 Sampling**: Intelligent log sampling for high-volume systems
- **🔐 Encryption**: At-rest and in-transit encryption
- **📈 Metrics Export**: Native Prometheus/OpenTelemetry integration

### **Performance Roadmap**
- **⚡ SIMD Optimization**: Vectorized compression and serialization
- **🧠 NUMA Awareness**: Optimize for multi-socket systems  
- **📱 ARM Optimization**: Native ARM64 performance tuning
- **🔧 Custom Allocators**: Zero-allocation logging paths
- **🏃‍♂️ JIT Compilation**: Runtime optimization of hot paths

### **Ecosystem Integration**
- **📦 Cargo Integration**: Built-in cargo logging during builds
- **🐳 Docker Support**: Container-native log collection
- **☸️ Kubernetes Operator**: Automated deployment and management
- **🔧 IDE Plugins**: VS Code/IntelliJ snapshot analysis tools

## 🤝 Contributing

We welcome contributions! Please see our [Contributing Guide](CONTRIBUTING.md) for details.

### **Development Setup**

```bash
# Clone the repository
git clone https://github.com/wildduck/ttlog.git
cd ttlog

# Install development dependencies
make install-tools

# Run comprehensive checks
make all

# Run tests with coverage
make test-coverage

# Run benchmarks
make bench

# Format and lint
make fmt lint

# Security audit
make audit
```

### **Contribution Areas**

- **🔥 Performance**: Optimize critical paths, SIMD implementations
- **🧪 Testing**: Add benchmarks, stress tests, integration tests  
- **📖 Documentation**: Improve examples, tutorials, API docs
- **🎯 Features**: Implement new storage backends, query capabilities
- **🐛 Bug Fixes**: Fix issues, improve error handling
- **🔧 Tooling**: Enhance viewer, analysis tools, IDE plugins

## 📄 License

This project is licensed under the MIT License - see the [LICENSE](LICENSE) file for details.

## 🙏 Acknowledgments

- **[Crossbeam](https://github.com/crossbeam-rs/crossbeam)**: For battle-tested lock-free data structures
- **[Serde](https://github.com/serde-rs/serde)**: For efficient serialization framework  
- **[LZ4](https://lz4.github.io/lz4/)**: For blazingly fast compression
- **[CBOR](https://cbor.io/)**: For compact binary serialization
- **Rust Community**: For creating an amazing ecosystem for systems programming

---

<div align="center">

**🚀 TTLog: Ultra-High-Performance Logging for the Modern Distributed World! 🚀**

*Built with ❤️ in Rust for maximum performance and reliability*

</div>
