# TTLog Examples

This directory contains comprehensive examples demonstrating how to use the `ttlog` library in various real-world scenarios. Each example showcases different aspects of the library's capabilities, from basic usage to complex distributed systems.

## 📚 Available Examples

### 🟢 [ttlog-simple](./ttlog-simple/) - Basic Usage Examples
**Perfect for beginners** - Learn the fundamentals of TTLog with step-by-step examples covering:
- Basic logging setup and configuration
- Structured logging patterns
- High-volume logging scenarios
- Multi-threaded logging
- Panic handling and recovery
- Error scenario management
- Custom service integration

**Best for**: Learning TTLog basics, understanding core concepts, and getting started quickly.

### 🔵 [ttlog-server](./ttlog-server/) - Web Server Integration
**Real-world HTTP server example** - See TTLog integrated into a web application:
- HTTP request/response logging
- Authentication and authorization events
- Performance monitoring
- Error handling in web contexts
- Background task logging
- Request correlation

**Best for**: Web developers, API developers, and anyone building HTTP-based services.

### 🟡 [ttlog-filereader](./ttlog-filereader/) - Snapshot Analysis Tool
**Utility for analyzing TTLog snapshots** - Read and analyze snapshot files:
- Decompress and decode snapshot files
- Display event details and statistics
- Analyze event patterns and distributions
- List available snapshots
- Programmatic snapshot reading

**Best for**: Debugging, analysis, and understanding what's captured in snapshots.

### 🔴 [ttlog-complex](./ttlog-complex/) - Distributed System Simulation
**Enterprise-scale example** - The most comprehensive demonstration featuring:
- 12 microservices with inter-service communication
- Circuit breakers and retry logic
- Database connection pooling
- Message queue processing
- Load balancing and health checks
- Chaos engineering and failure injection
- Distributed tracing with correlation IDs
- Security audit logging
- Business intelligence events
- Performance profiling and monitoring

**Best for**: Understanding TTLog in complex, production-like environments.

## 🚀 Quick Start

### Prerequisites
```bash
# Ensure you have Rust installed
rustc --version

# Clone the repository (if not already done)
git clone <repository-url>
cd ttlog
```

### Running Examples

#### 1. Start with Simple Examples
```bash
cd examples/ttlog-simple
cargo run
```

#### 2. Try the Web Server Example
```bash
cd examples/ttlog-server
cargo run
```

#### 3. Analyze Generated Snapshots
```bash
cd examples/ttlog-filereader
cargo run -- --list
cargo run /tmp/ttlog-<pid>-<timestamp>-<reason>.bin
```

#### 4. Experience the Complex System
```bash
cd examples/ttlog-complex
cargo run
```

## 📊 Understanding the Output

### Snapshot Files
All examples generate compressed snapshot files in `/tmp/` with the naming pattern:
```
/tmp/ttlog-<pid>-<timestamp>-<reason>.bin
```

### Example Snapshot Names
- `ttlog-12345-20250101123456-startup.bin`
- `ttlog-12345-20250101123457-panic.bin`
- `ttlog-12345-20250101123458-high_load.bin`
- `ttlog-12345-20250101123459-server_shutdown.bin`

### Analyzing Snapshots
```bash
# List all available snapshots
cd examples/ttlog-filereader
cargo run -- --list

# Read a specific snapshot
cargo run /tmp/ttlog-12345-20250101123456-startup.bin

# Count total snapshots
ls -la /tmp/ttlog-*.bin | wc -l
```

## 🎯 Learning Path

### For Beginners
1. **Start with `ttlog-simple`** - Learn basic concepts
2. **Try `ttlog-server`** - See real-world integration
3. **Use `ttlog-filereader`** - Learn to analyze output
4. **Explore `ttlog-complex`** - Understand advanced patterns

### For Experienced Developers
1. **Skip to `ttlog-complex`** - See enterprise patterns
2. **Use `ttlog-filereader`** - For debugging and analysis
3. **Reference `ttlog-server`** - For web integration patterns
4. **Review `ttlog-simple`** - For specific implementation details

## 🔧 Customization

### Modifying Examples
Each example can be customized by:
- Adjusting buffer sizes in `Trace::init(capacity, snapshot_threshold)`
- Changing snapshot trigger conditions
- Modifying logging patterns and message formats
- Adding custom business logic

### Example Customization
```rust
// In any example, you can modify the trace initialization:
let trace_system = Trace::init(10000, 1000); // Larger buffer, higher threshold

// Add custom snapshot triggers:
if error_condition {
    trace_system.request_snapshot("custom_error");
}
```

## 🧪 Testing Examples

### Running Tests
```bash
# Test all examples
cargo test --workspace

# Test specific example
cd examples/ttlog-simple
cargo test
```

### Verification
After running examples, verify they worked by:
```bash
# Check for snapshot files
ls -la /tmp/ttlog-*.bin

# Analyze snapshots
cd examples/ttlog-filereader
cargo run -- --list
```

## 📈 Performance Considerations

### Buffer Sizing
- **Small applications**: 1,000-5,000 events
- **Medium applications**: 5,000-20,000 events  
- **Large applications**: 20,000-100,000 events
- **Enterprise systems**: 100,000+ events

### Snapshot Frequency
- **Development**: Every few minutes or on errors
- **Production**: On critical events, errors, or state changes
- **Debugging**: On demand or at specific checkpoints

## 🛠️ Troubleshooting

### Common Issues

#### No Snapshot Files Generated
```bash
# Check if /tmp is writable
ls -la /tmp/

# Verify the process has write permissions
touch /tmp/test-file
```

#### Snapshot Files Are Empty
- Check if events are being logged
- Verify the buffer capacity is appropriate
- Ensure snapshot triggers are being called

#### Performance Issues
- Reduce buffer size if memory is constrained
- Increase snapshot threshold to reduce I/O
- Use more selective snapshot triggers

### Debug Mode
Enable debug logging to see what's happening:
```bash
RUST_LOG=debug cargo run
```

## 📚 Additional Resources

### Documentation
- [TTLog Library Documentation](../README.md)
- [API Reference](../src/lib.rs)
- [Configuration Guide](../docs/configuration.md)

### Community
- [Issues and Bug Reports](https://github.com/your-repo/issues)
- [Feature Requests](https://github.com/your-repo/issues)
- [Discussions](https://github.com/your-repo/discussions)

## 🎉 Contributing

### Adding New Examples
1. Create a new directory in `examples/`
2. Include a `Cargo.toml` with TTLog dependency
3. Write comprehensive example code
4. Add a detailed README.md
5. Include tests for the example

### Example Template
```bash
examples/ttlog-your-example/
├── Cargo.toml
├── README.md
├── src/
│   └── main.rs
└── tests/
    └── integration_tests.rs
```

---

**Happy Logging! 🚀**

These examples demonstrate TTLog's versatility from simple applications to complex distributed systems. Start with the simple examples and work your way up to the complex distributed system simulation to see the full power of TTLog in action. 