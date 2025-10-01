# WARP.md

This file provides guidance to WARP (warp.dev) when working with code in this repository.

## Project Overview

TTLog is a high-performance, lock-free structured logging library built for Rust applications that need minimal overhead logging with crash recovery capabilities. It achieves 318M events/sec throughput using ring buffers and thread-local string interning, with automatic compressed snapshots for post-mortem debugging.

## Common Development Commands

### Building and Testing
```bash
# Run complete development workflow (check, format, lint, test)
make all

# Build all crates in the workspace
make build

# Run all tests
make test

# Check compilation without building
make check

# Format code with project-specific rustfmt config
make format

# Run clippy linting
make lint

# Clean all build artifacts
make clean
```

### Individual Crate Operations
```bash
# Work on specific crates
make build-ttlog    # Main logging library
make build-view     # Log viewer/analysis tool
make test-ttlog     # Test main library
make test-view      # Test viewer

# Run examples
make run-simple     # Basic usage example
make run-server     # Server logging example
make run-complex    # Advanced usage patterns
```

### Performance Analysis
```bash
# Run comprehensive benchmarks
make bench

# Quick benchmarks for development
make bench-quick

# Stress testing with high load
make bench-stress

# Generate detailed benchmark reports
make benchmark-report

# Memory profiling (requires heaptrack)
make mem-profile

# CPU profiling (requires flamegraph)
make cpu-profile
```

### Snapshot Analysis
```bash
# View crash snapshots and logs
ttlog-view /tmp/ttlog-*.bin

# Filter and analyze snapshots
ttlog-view /tmp/ttlog-*.bin --filter level=ERROR --limit 100

# Export for further analysis
ttlog-view /tmp/ttlog-*.bin --format json > analysis.json
```

## Architecture Overview

TTLog uses a sophisticated multi-threaded architecture designed for zero-allocation logging:

### Core Components
- **Lock-Free Ring Buffer**: Uses crossbeam's ArrayQueue for contention-free event storage
- **String Interning**: Thread-local interning with RwLock fallback reduces allocations
- **Writer Thread**: Separate thread handles all I/O operations to prevent blocking
- **Listener System**: Pluggable output handlers (stdout, file, custom)
- **Snapshot System**: Automatic crash recovery with CBOR + LZ4 compression

### Event Flow
```
Application → Log Macros → Ring Buffer → Writer Thread → {Snapshots, Listeners}
```

### Key Design Decisions
- **Bounded buffers** prevent unbounded memory growth
- **Fire-and-forget logging** - never blocks application threads
- **Separate snapshot and listener buffers** for different use cases
- **Atomic file operations** prevent corrupted snapshots
- **24-byte event structure** for memory efficiency

## Workspace Structure

This is a Cargo workspace with specialized crates:
- `ttlog/` - Core logging library with lock-free data structures
- `ttlog-view/` - TUI viewer for analyzing compressed snapshots
- `ttlog-macros/` - Procedural macros for convenient structured logging
- `ttlog-benches/` - Comprehensive benchmark suite
- `examples/` - Usage patterns from simple to complex scenarios

## Development Guidelines

### Performance Considerations
- The library targets 318M+ events/sec throughput
- Event size is exactly 24 bytes - changes require careful analysis
- String interning effectiveness depends on string reuse patterns
- Buffer sizing guidelines are provided for different workload types

### Testing Strategy
- Tests are organized in `__test__` modules for clear separation
- Benchmarks use statistical analysis with multiple trials
- Stress tests validate behavior under extreme loads
- End-to-end tests include actual I/O performance measurement

### Memory Management
- Ring buffers have fixed capacity - configure appropriately
- String interning pools can grow - monitor in long-running applications
- Snapshot compression ratios vary by event content patterns

### Build Configuration
- Uses custom rustfmt configuration in `rustfmt.toml`
- Release profile is configured for debugging (opt-level=0, debug=true)
- Workspace dependencies centralized in root `Cargo.toml`

## Integration Notes

The library provides multiple integration options:
- Direct initialization with `Trace::init()`
- Quick setup functions like `init_stdout()` and `init_file()`
- Tracing ecosystem compatibility via subscriber layers
- Custom listener implementation for specialized outputs

Buffer sizing should match workload characteristics:
- High-throughput: 10M+ ring buffer, 1M+ channel capacity
- General web services: 1M ring buffer, 100K channel capacity  
- Resource-constrained: 10K ring buffer, 1K channel capacity

The snapshot system automatically handles crashes but manual snapshots can be requested for debugging specific application states.