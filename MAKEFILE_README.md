# Makefile Usage Guide

This project includes a comprehensive Makefile that automates common development tasks for the TTLog high-performance distributed logging library.

## 🚀 Quick Start

```bash
# Show all available commands
make help

# Run the complete development workflow
make all

# Install required development tools
make install-tools
```

## 📦 Core Commands

### Development Workflow
- `make all` - Run check, format, lint, and test (recommended before commits)
- `make dev` - Quick development checks (check, format, lint)

### Building
- `make build` - Build all crates
- `make release` - Build release versions
- `make clean` - Clean all build artifacts

### Testing
- `make test` - Run all tests
- `make test-ttlog` - Test only the main ttlog crate
- `make test-view` - Test only the ttlog-view crate
- `make test-event` - Test only the ttlog-event crate
- `make test-examples` - Test all example crates

### Code Quality
- `make check` - Check all crates compile
- `make format` - Format all code with rustfmt
- `make lint` - Run clippy linting
- `make check-tests` - Verify all tests are in `__test__` modules

### Documentation
- `make docs` - Generate and open documentation
- `make docs-serve` - Generate documentation and serve locally
- `make docs-build` - Build documentation without opening

## 📊 Benchmarking Commands

### Comprehensive Benchmarks
```bash
make bench              # Run all benchmarks with full configuration
make bench-quick        # Run quick benchmarks for faster feedback
make bench-distributed  # Run distributed system benchmarks
make bench-stress       # Run stress tests
make benchmark-report   # Generate comprehensive benchmark report
```

### Performance Testing
```bash
make perf-test          # Run performance tests
make mem-profile        # Run memory profiling (requires heaptrack)
make cpu-profile        # Run CPU profiling (requires cargo-flamegraph)
```

## 🎯 Example Commands

### Running Examples
```bash
make run-simple         # Run simple logging example
make run-server         # Run server-side logging example
make run-complex        # Run complex async/distributed example
make run-filereader     # Run file reader example
```

### Viewer Tool
```bash
make install-viewer     # Install ttlog-view tool
make view-snapshots     # Open viewer to analyze snapshots
```

## 🔍 Analysis Commands

### Security & Dependencies
```bash
make audit              # Check for security vulnerabilities
make update             # Update dependencies
make outdated           # Check for outdated dependencies
```

### Workspace Health
```bash
make workspace-health   # Check workspace health and dependencies
make coverage           # Run tests with coverage (requires cargo-tarpaulin)
```

## 📁 Individual Crate Commands

You can also work with specific crates:

```bash
# Build specific crates
make build-ttlog
make build-view
make build-event

# Check specific crates
make check-ttlog
make check-view
make check-event

# Format specific crates
make format-ttlog
make format-view
make format-event

# Test specific crates
make test-ttlog
make test-view
make test-event
```

## 🛠️ Advanced Commands

### Release Management
```bash
make pre-release        # Run all pre-release checks
make release-build      # Clean build for release
make changelog          # Generate changelog (requires conventional-changelog)
```

### Maintenance
```bash
make clean-snapshots    # Clean snapshot files from /tmp
make history            # Export git history for documentation
```

## 📊 Benchmark Configuration

### Quick Mode
For faster development feedback:
```bash
export CRITERION_SAMPLE_SIZE=10
export CRITERION_MEASUREMENT_TIME=2000
export CRITERION_WARM_UP_TIME=500
make bench-quick
```

### Full Mode
For comprehensive performance analysis:
```bash
export CRITERION_SAMPLE_SIZE=30
export CRITERION_MEASUREMENT_TIME=10000
export CRITERION_WARM_UP_TIME=5000
make bench
```

## 🔧 Prerequisites

Make sure you have the following tools installed:

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
make install-tools

# Optional: Install profiling tools
cargo install flamegraph
sudo apt install heaptrack  # For memory profiling
```

## 📋 Test Organization

All tests in this project are properly organized in `__test__` modules within each source directory. The `make check-tests` command verifies this organization.

## 🏗️ Workspace Structure

This project uses Cargo workspaces to manage multiple crates:
- `ttlog` - Main library crate with lock-free ring buffer
- `ttlog-event` - Proc macros for convenient logging
- `ttlog-view` - Viewer and analysis tool
- `examples/*` - Comprehensive usage examples

## 🚀 Performance Testing Workflow

### Development Testing
```bash
# Quick performance check
make bench-quick

# Run specific stress tests
make bench-stress
```

### Production Testing
```bash
# Comprehensive benchmark suite
make benchmark-report

# Memory and CPU profiling
make mem-profile
make cpu-profile
```

### Continuous Integration
```bash
# Full CI pipeline
make pre-release
```

## 🔍 Troubleshooting

### Common Issues

1. **Nightly toolchain required for formatting**
   ```bash
   rustup toolchain install nightly
   rustup component add rustfmt --toolchain nightly
   ```

2. **Clippy not found**
   ```bash
   rustup component add clippy
   ```

3. **Workspace commands fail**
   - Ensure you're in the root directory
   - Check that `Cargo.toml` workspace is properly configured

4. **Benchmark failures**
   ```bash
   # Check system resources
   make workspace-health
   
   # Run with reduced load
   make bench-quick
   ```

5. **Profiling tools not found**
   ```bash
   # Install profiling tools
   cargo install flamegraph
   sudo apt install heaptrack
   ```

### Getting Help

```bash
make help  # Show all available commands
```

## 📈 Performance Targets

### Expected Performance Numbers

| System Type | Events/sec | Concurrent Threads | Buffer Capacity |
|-------------|------------|-------------------|-----------------|
| High-End (32+ cores, 64GB+ RAM) | 500K - 2M | 256 - 1024 | 100K - 1M |
| Mid-Range (8-16 cores, 16-32GB RAM) | 100K - 500K | 64 - 256 | 10K - 100K |
| Standard (4-8 cores, 8-16GB RAM) | 50K - 200K | 16 - 64 | 1K - 10K |

### Success Criteria
- **Throughput**: >100K events/sec on standard hardware
- **Scalability**: Linear scaling up to 64 threads
- **Memory**: <1KB per event under normal conditions
- **Latency**: <1ms for single event operations

## 🤝 Contributing

When contributing to this project:

1. Run `make all` before submitting PRs
2. Ensure all tests pass with `make test`
3. Format code with `make format`
4. Check for linting issues with `make lint`
5. Verify test organization with `make check-tests`
6. Run benchmarks with `make bench-quick`
7. Check workspace health with `make workspace-health`

## 📚 Additional Resources

- **TTLog Documentation**: Check the main README.md for detailed usage
- **Criterion.rs Documentation**: https://bheisler.github.io/criterion.rs/
- **Rust Performance Book**: https://nnethercote.github.io/perf-book/
- **Crossbeam Documentation**: https://docs.rs/crossbeam/

---

**🚀 This Makefile provides a complete development workflow for TTLog!** 