# TTLog - High-Performance Logging Library

## Overview

TTLog is a high-performance, non-blocking logging library for Rust applications that maintains a circular buffer of log events in memory and automatically creates compressed snapshots to disk under specific conditions. It's designed for production systems where logging performance is critical and post-mortem debugging capabilities are essential.

## Key Features

- **Zero-Copy Ring Buffer**: Events are stored in a fixed-size circular buffer that automatically overwrites old events
- **Non-Blocking Logging**: Uses crossbeam channels with `try_send` to prevent blocking the main application
- **Automatic Snapshots**: Creates compressed snapshots on panics, periodic intervals, or manual requests
- **Tracing Integration**: Implements tracing-subscriber layers for seamless integration with the Rust tracing ecosystem
- **Compressed Storage**: Uses CBOR serialization + LZ4 compression for efficient disk storage
- **Atomic File Operations**: Ensures snapshot files are written atomically to prevent corruption

## Architecture

### Core Components

```
┌─────────────────┐    ┌──────────────────┐    ┌─────────────────┐
│   Application   │    │   BufferLayer    │    │  Writer Thread  │
│                 │    │                  │    │                 │
│  tracing::info! │───▶│  Captures Events │───▶│   Ring Buffer   │
│                 │    │                  │    │                 │
└─────────────────┘    └──────────────────┘    └─────────────────┘
                                                         │
                                                         ▼
                                               ┌─────────────────┐
                                               │   Snapshot      │
                                               │   Creation      │
                                               │                 │
                                               │ CBOR + LZ4 +    │
                                               │ Atomic Write    │
                                               └─────────────────┘
```

### Data Flow

1. **Event Capture**: The `BufferLayer` intercepts tracing events and converts them to internal `Event` structs
2. **Channel Transport**: Events are sent through a bounded crossbeam channel to the writer thread
3. **Ring Buffer Storage**: The writer thread maintains a `RingBuffer` that stores the most recent N events
4. **Snapshot Triggers**: Snapshots are created on:
   - Application panics (via panic hook)
   - Periodic intervals (configurable, default 60 seconds)
   - Manual requests via `request_snapshot()`
5. **Compression & Storage**: Snapshots are serialized to CBOR, compressed with LZ4, and written atomically to `/tmp/`

## Module Breakdown

### `buffer` Module - Ring Buffer Implementation

```rust
pub struct RingBuffer<T: Clone> {
    data: VecDeque<T>,
    capacity: usize,
}
```

**Purpose**: Implements a fixed-size circular buffer using `VecDeque` for efficient front/back operations.

**Key Methods**:
- `new(capacity)`: Creates empty buffer with specified capacity
- `push(item)`: Adds item, removes oldest if at capacity
- `take_snapshot()`: Efficiently extracts all current items, leaving empty buffer
- `iter()`: Returns iterator over current items

**Design Decision**: Uses `std::mem::replace` in `take_snapshot()` to avoid per-element `pop_front()` overhead.

### `event` Module - Log Event Structure

```rust
pub struct Event {
    pub timestamp: u64,
    pub level: String, 
    pub message: String,
    // Additional fields commented out for minimal overhead
}
```

**Purpose**: Defines the core log event structure with minimal fields for performance.

**Key Features**:
- Timestamp stored as milliseconds since epoch
- Level stored as string for flexibility
- Commented out fields (target, span_id, fields, etc.) show extensibility options
- Implements serialization/deserialization for persistence

**Design Decision**: Minimal field set reduces memory overhead and serialization cost.

### `trace` Module - Main Orchestration

```rust
pub struct Trace {
    sender: Sender<Message>,
}

pub enum Message {
    Event(Event),
    SnapshotImmediate(String), // reason
    FlushAndExit,
}
```

**Purpose**: Main entry point that orchestrates the logging system.

**Key Responsibilities**:
- Spawns dedicated writer thread
- Manages crossbeam channel for event transport
- Registers tracing subscriber globally
- Provides API for manual snapshot requests

**Writer Loop Design**:
- Single background thread owns the ring buffer
- Processes events and snapshot requests sequentially
- Implements periodic flushing with configurable intervals
- Handles graceful shutdown via `FlushAndExit` message

### `trace_layer` Module - Tracing Integration

```rust
pub struct BufferLayer {
    sender: Sender<Message>,
}

impl<T> Layer<T> for BufferLayer where T: Subscriber + for<'a> LookupSpan<'a>
```

**Purpose**: Implements tracing-subscriber `Layer` trait for seamless integration.

**Key Features**:
- Extracts timestamp, level, and message from tracing events
- Uses `MessageVisitor` to handle different field types
- Non-blocking `try_send()` - drops events if channel is full
- Minimal overhead event conversion

**Design Decision**: Intentionally drops events when channel is full rather than blocking application threads.

### `snapshot` Module - Persistence Layer

```rust
pub struct Snapshot {
    pub service: String,
    pub hostname: String, 
    pub pid: u32,
    pub created_at: String,
    pub reason: String,
    pub events: Vec<Event>,
}
```

**Purpose**: Handles serialization, compression, and atomic file writing.

**Process**:
1. Takes snapshot from ring buffer
2. Adds metadata (hostname, PID, timestamp, reason)
3. Serializes to CBOR for compact binary format
4. Compresses with LZ4 for speed/size balance
5. Writes atomically using temp file + rename

**File Naming**: `/tmp/ttlog-{pid}-{timestamp}-{reason}.bin`

### `panic_hook` Module - Crash Recovery

```rust
pub struct PanicHook {}

impl PanicHook {
    pub fn install(sender: Sender<Message>)
}
```

**Purpose**: Installs global panic hook to capture logs during crashes.

**Operation**:
- Registers with `std::panic::set_hook`
- On panic, sends `SnapshotImmediate` message with reason "panic"
- Uses `try_send()` to avoid blocking during panic handling

## Usage Guide

### Basic Setup

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

### Manual Snapshots

```rust
// Request immediate snapshot
trace.request_snapshot("checkpoint");

// This will create: /tmp/ttlog-{pid}-{timestamp}-checkpoint.bin
```

### Configuration Options

The library uses hardcoded defaults but can be customized by modifying the source:

- **Ring Buffer Capacity**: Number of events to keep in memory
- **Channel Capacity**: Bounded channel size for event transport  
- **Periodic Interval**: How often to create automatic snapshots (default: 60 seconds)
- **Output Directory**: Currently hardcoded to `/tmp/`
- **Service Name**: Currently hardcoded to "my_service"

## Performance Characteristics

### Memory Usage
- **Ring Buffer**: `capacity * sizeof(Event)` 
- **Channel Buffer**: `channel_capacity * sizeof(Message)`
- **Overhead**: ~40-50 bytes per event (timestamp + level + message)

### CPU Overhead
- **Logging Path**: Single allocation + channel send
- **Writer Thread**: Minimal - only processes events and occasional snapshots
- **Snapshot Creation**: CPU burst during CBOR serialization and LZ4 compression

### Backpressure Handling
- **Channel Full**: Events are dropped silently to prevent blocking
- **Disk I/O**: Performed on separate thread, doesn't block logging

## File Format

Snapshot files use the following format:

1. **Serialization**: CBOR (Concise Binary Object Representation)
2. **Compression**: LZ4 with default compression mode
3. **Structure**:
   ```rust
   Snapshot {
       service: String,      // Service identifier
       hostname: String,     // Machine hostname  
       pid: u32,            // Process ID
       created_at: String,   // Timestamp (YYYYMMDDHHMMSS)
       reason: String,       // Why snapshot was taken
       events: Vec<Event>,   // Actual log events
   }
   ```

### Reading Snapshots

```rust
use std::fs;
use lz4::block::decompress;

// Read and decompress snapshot file
let compressed = fs::read("/tmp/ttlog-1234-20240814123045-panic.bin")?;
let cbor_data = decompress(&compressed, None)?;  
let snapshot: Snapshot = serde_cbor::from_slice(&cbor_data)?;

// Access events
for event in snapshot.events {
    println!("{}: {} - {}", event.timestamp, event.level, event.message);
}
```

## Design Philosophy

### 1. **Performance First**
- Non-blocking logging path prevents application slowdown
- Ring buffer with fixed size prevents unbounded memory growth
- Minimal allocations in hot path

### 2. **Crash Recovery**
- Automatic panic hook ensures logs are captured during crashes
- Atomic file operations prevent corrupted snapshots
- Recent events are always preserved in ring buffer

### 3. **Production Ready**
- Handles backpressure gracefully by dropping events
- Periodic flushing prevents log loss during long-running processes
- Compressed storage reduces disk usage

### 4. **Observability**
- Rich metadata in snapshots (hostname, PID, timestamp, reason)
- Human-readable event structure  
- Integration with standard tracing ecosystem

## Limitations & Considerations

### Current Limitations
1. **Fixed Output Location**: Snapshots always written to `/tmp/`
2. **Limited Event Fields**: Only timestamp, level, message captured
3. **No Log Levels**: All tracing events captured regardless of level
4. **Single Service**: Service name is hardcoded
5. **No Rotation**: Snapshot files accumulate indefinitely

### Production Considerations
1. **Disk Space**: Monitor `/tmp/` usage, especially with frequent snapshots
2. **File Cleanup**: Implement external cleanup for old snapshot files  
3. **Channel Sizing**: Tune channel capacity based on logging volume
4. **Ring Buffer Size**: Balance memory usage vs. event retention
5. **Error Handling**: Snapshot creation errors only logged to stderr

## Testing

The library includes comprehensive tests for all major components:

- **Buffer Tests**: Ring buffer overflow, iteration, snapshot extraction
- **Event Tests**: Serialization/deserialization roundtrip
- **Integration Tests**: End-to-end logging with file creation verification
- **Concurrency Tests**: Multi-threaded logging scenarios
- **Panic Tests**: Verification that panic hook creates snapshot files

Run tests with:
```bash
cargo test
```

## Future Enhancements

Potential improvements based on the current design:

1. **Configurable Output**: Support different output directories and file formats
2. **Log Level Filtering**: Respect tracing level configuration  
3. **Field Extraction**: Capture additional structured fields from tracing events
4. **Rotation Policy**: Automatic cleanup of old snapshot files
5. **Multiple Services**: Support for service-specific configuration
6. **Metrics Integration**: Export metrics about dropped events, snapshot frequency
7. **Remote Storage**: Support for uploading snapshots to cloud storage
8. **Event Sampling**: Configurable sampling for high-volume scenarios

## Conclusion

TTLog provides a robust foundation for high-performance logging with automatic crash recovery. Its design prioritizes application performance while ensuring critical log data is preserved and accessible for debugging. The modular architecture makes it extensible for future enhancements while maintaining the core performance characteristics.

