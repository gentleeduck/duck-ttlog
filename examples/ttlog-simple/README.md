# TTLog Simple Examples

A comprehensive collection of basic TTLog usage examples designed for beginners and those learning the library. This example demonstrates fundamental concepts through practical, easy-to-understand scenarios.

## 🎯 What You'll Learn

This example covers all the essential TTLog concepts through 8 focused modules:

### 📋 Example Modules

#### 1. **Basic Logging** (`example_simple`)
- Minimal TTLog setup and configuration
- Basic event logging with different levels
- Understanding the core logging flow

#### 2. **Structured Logging** (`example_structured_logging`)
- Logging with structured data and context
- Using tracing macros effectively
- Best practices for message formatting

#### 3. **High Volume Logging** (`example_high_volume_logging`)
- Handling large numbers of events
- Buffer management and overflow scenarios
- Performance considerations for high-throughput applications

#### 4. **Multi-threaded Logging** (`example_multithreaded_logging`)
- Thread-safe logging across multiple threads
- Concurrent event handling
- Race condition prevention

#### 5. **Panic Handling** (`example_panic_handling`)
- Automatic snapshot creation on panics
- Panic recovery and debugging
- Using panic hooks for crash analysis

#### 6. **Error Scenarios** (`example_error_scenarios`)
- Logging different types of errors
- Error context and debugging information
- Error recovery patterns

#### 7. **Custom Service Integration** (`example_custom_service`)
- Integrating TTLog into existing services
- Custom snapshot triggers
- Service-specific logging patterns

#### 8. **Basic Logging Patterns** (`example_basic_logging`)
- Common logging patterns and idioms
- Event categorization and organization
- Logging best practices

## 🚀 Quick Start

### Prerequisites
```bash
# Ensure you have Rust installed
rustc --version

# Navigate to the example directory
cd examples/ttlog-simple
```

### Running the Examples
```bash
# Run all examples
cargo run

# Run with verbose output
RUST_LOG=debug cargo run

# Run specific example (modify main.rs to run only one)
cargo run
```

## 📊 Expected Output

When you run the examples, you'll see output like:

```
TTLog Library Examples
=====================

=== Simple Example ===
[INFO] Application started
[INFO] Processing request 1
[WARN] High memory usage detected
[ERROR] Database connection failed
[INFO] Application shutdown

=== Structured Logging Example ===
[INFO] user.login user_id=123 ip=192.168.1.1 ms=45
[INFO] order.create order_id=abc customer=456 total=99.99
[ERROR] db.query fail table=users error=timeout ms=150

=== High Volume Example ===
[INFO] Generated 1000 events in 50ms
[INFO] Buffer utilization: 85%

=== Multi-threaded Example ===
[INFO] Thread 1: Processing batch 1
[INFO] Thread 2: Processing batch 2
[INFO] All threads completed

=== Panic Handling Example ===
[INFO] Starting panic test
[ERROR] Panic occurred: test panic
[INFO] Panic hook triggered snapshot

=== Error Scenarios Example ===
[ERROR] Network timeout after 5000ms
[ERROR] Database connection lost
[WARN] Retrying operation (attempt 2/3)

=== Custom Service Example ===
[INFO] service.startup version=1.0.0 port=8080
[INFO] service.request method=GET path=/api/users ms=25
[INFO] service.shutdown reason=graceful

=== Basic Logging Example ===
[INFO] Application initialized
[DEBUG] Configuration loaded
[INFO] Server listening on port 3000

=== All Examples Completed ===
Check /tmp/ directory for generated snapshot files:
  ls -la /tmp/ttlog-*.bin
```

## 📁 Generated Files

After running the examples, check `/tmp/` for snapshot files:

```bash
# List all generated snapshots
ls -la /tmp/ttlog-*.bin

# Example snapshot names:
# ttlog-12345-20250101123456-simple_example.bin
# ttlog-12345-20250101123457-structured_logging.bin
# ttlog-12345-20250101123458-high_volume.bin
# ttlog-12345-20250101123459-multithreaded.bin
# ttlog-12345-20250101123460-panic_test.bin
# ttlog-12345-20250101123461-error_scenarios.bin
# ttlog-12345-20250101123462-custom_service.bin
# ttlog-12345-20250101123463-basic_logging.bin
```

## 🔍 Analyzing the Output

### Using the File Reader
```bash
# Navigate to the file reader example
cd ../ttlog-filereader

# List all available snapshots
cargo run -- --list

# Read a specific snapshot
cargo run /tmp/ttlog-12345-20250101123456-simple_example.bin
```

### Understanding Snapshot Content
Each snapshot contains:
- **Service information**: Name, hostname, PID
- **Creation details**: Timestamp, reason for snapshot
- **Event list**: All captured events with timestamps, levels, and messages
- **Metadata**: Event counts, time ranges, target information

## 🎓 Learning Path

### For Complete Beginners
1. **Start with `example_simple`** - Understand basic setup
2. **Try `example_basic_logging`** - Learn common patterns
3. **Explore `example_structured_logging`** - See structured data logging
4. **Experiment with `example_panic_handling`** - Understand error recovery

### For Developers with Some Experience
1. **Focus on `example_high_volume_logging`** - Performance considerations
2. **Study `example_multithreaded_logging`** - Concurrency patterns
3. **Review `example_error_scenarios`** - Error handling best practices
4. **Examine `example_custom_service`** - Integration patterns

## 🔧 Customization Examples

### Modifying Buffer Size
```rust
// In any example, change the buffer capacity:
let trace_system = Trace::init(5000, 500); // 5000 events, snapshot at 500
```

### Adding Custom Snapshot Triggers
```rust
// Add custom snapshot points:
if error_count > 10 {
    trace_system.request_snapshot("high_error_rate");
}

if memory_usage > 80.0 {
    trace_system.request_snapshot("high_memory_usage");
}
```

### Custom Logging Patterns
```rust
// Structured logging with business context:
info!(
    user_id = user.id,
    action = "login",
    ip_address = request.ip,
    user_agent = request.user_agent,
    "User authentication"
);
```

## 🧪 Testing Individual Examples

### Running Specific Examples
To run only one example, modify `main.rs`:

```rust
fn main() {
    println!("TTLog Simple Example");
    println!("===================");

    // Run only the simple example
    example_simple();
    
    // Comment out other examples
    // example_basic_logging();
    // example_structured_logging();
    // ...
}
```

### Adding Tests
```rust
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_simple_example_creates_snapshot() {
        example_simple();
        
        // Verify snapshot was created
        let entries: Vec<_> = std::fs::read_dir("/tmp")
            .unwrap()
            .filter_map(|e| e.ok())
            .filter(|e| e.file_name().to_string_lossy().contains("simple"))
            .collect();
            
        assert!(!entries.is_empty());
    }
}
```

## 📈 Performance Tips

### Buffer Sizing Guidelines
- **Development**: 1,000-2,000 events
- **Testing**: 5,000-10,000 events
- **Production**: 10,000+ events (based on event volume)

### Snapshot Frequency
- **Development**: Every few minutes or on errors
- **Testing**: On specific test completion
- **Production**: On critical events or state changes

### Memory Considerations
- Each event uses ~100-200 bytes
- Buffer capacity directly affects memory usage
- Monitor memory usage in high-volume scenarios

## 🛠️ Troubleshooting

### Common Issues

#### No Output Generated
```bash
# Check if TTLog is properly initialized
# Verify tracing macros are being used
# Ensure panic hook is installed
```

#### Snapshot Files Missing
```bash
# Check /tmp directory permissions
ls -la /tmp/

# Verify snapshot triggers are called
# Check for error messages in output
```

#### Performance Issues
```bash
# Reduce buffer size if memory constrained
# Increase snapshot threshold to reduce I/O
# Use more selective logging levels
```

### Debug Mode
```bash
# Enable debug logging
RUST_LOG=debug cargo run

# Enable trace logging for maximum detail
RUST_LOG=trace cargo run
```

## 📚 Next Steps

After completing these examples:

1. **Try the Web Server Example** (`../ttlog-server/`) - Real-world HTTP integration
2. **Use the File Reader** (`../ttlog-filereader/`) - Analyze your snapshots
3. **Explore the Complex Example** (`../ttlog-complex/`) - Enterprise patterns
4. **Integrate TTLog into your own project** - Apply what you've learned

## 🎉 Key Takeaways

By completing these examples, you'll understand:

- ✅ How to set up TTLog in any Rust application
- ✅ Best practices for structured logging
- ✅ Handling high-volume logging scenarios
- ✅ Thread-safe logging patterns
- ✅ Panic recovery and debugging
- ✅ Error handling and recovery
- ✅ Custom service integration
- ✅ Performance optimization techniques

---

**Ready to start? Run `cargo run` and explore the power of TTLog! 🚀** 