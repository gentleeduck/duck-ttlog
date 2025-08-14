# TTLog Complex Example - Distributed Microservices System

This is the most comprehensive example of using the `ttlog` library in a real-world distributed system scenario.

## 🚀 Features Demonstrated

### Core TTLog Integration
- **Global Tracing Setup**: `Trace::init(50000, 10000)` with large buffer capacity
- **Panic Hook Integration**: Automatic snapshot creation on panics
- **Strategic Snapshot Points**: Snapshots triggered by business events, errors, and system state changes
- **Real-time Buffer Inspection**: Live buffer access for debugging

### Distributed System Simulation
- **12 Microservices**: User, Product, Order, Payment, Inventory, Notification, Analytics, Audit, Recommendation, Search, Pricing, Shipping
- **Inter-service Communication**: Message passing with correlation IDs and distributed tracing
- **Circuit Breakers**: Automatic failure detection and recovery
- **Database Connection Pooling**: Simulated with realistic failure scenarios
- **Load Balancing**: Health checks and instance selection

### Advanced Observability
- **Distributed Tracing**: Correlation IDs across service boundaries
- **Performance Monitoring**: Response times, success rates, resource usage
- **Security Audit Logging**: Risk scoring and anomaly detection
- **Business Intelligence**: Analytics events and metrics collection
- **Chaos Engineering**: Random failure injection for resilience testing

### Real-world Scenarios
- **E-commerce Workload**: User registrations, product searches, order placements
- **Peak Load Handling**: Time-based load multipliers (morning, lunch, evening peaks)
- **Error Scenarios**: Database failures, network timeouts, service unavailability
- **Security Events**: Injection attacks, rate limiting, authentication bypass attempts

## 🏃‍♂️ Running the Example

```bash
# Build and run
cargo run --bin ttlog-complex

# The system will run for 10 minutes, generating:
# - Complex inter-service interactions
# - Multiple snapshot files in /tmp/
# - Real-time logging output
# - System tests and benchmarks
```

## 📊 Generated Snapshots

The example creates numerous snapshot files in `/tmp/` with names like:
- `ttlog-<pid>-<timestamp>-startup.bin`
- `ttlog-<pid>-<timestamp>-panic.bin`
- `ttlog-<pid>-<timestamp>-high_load_scenario.bin`
- `ttlog-<pid>-<timestamp>-security_risk_payment_processed.bin`
- `ttlog-<pid>-<timestamp>-chaos_Random_Service_Failure.bin`

## 🔍 Analyzing Snapshots

Use the companion `dump` tool to analyze snapshot files:

```bash
# Build the dump tool
cargo build --bin dump

# Convert snapshot to JSON for analysis
./target/debug/dump /tmp/ttlog-12345-20250101123456-startup.bin | head -20

# Count total snapshots
ls -la /tmp/ttlog-*.bin | wc -l

# Find latest snapshots
ls -la /tmp/ttlog-*.bin | tail -10
```

## 🎯 Key TTLog Usage Patterns

### 1. Strategic Snapshot Points
```rust
// On critical business events
trace_system.request_snapshot("order_placed");

// On error conditions
trace_system.request_snapshot("payment_failed");

// On system state changes
trace_system.request_snapshot("service_startup");
```

### 2. Concise, Self-contained Messages
```rust
// Good: All context in message
info!("auth.login ok user=123 ip=1.2.3.4 ms=32");

// Good: Business events with correlation
info!("order.place order_id=abc customer=456 total=99.99");

// Good: Error with context
error!("db.query fail table=users error=connection_lost ms=150");
```

### 3. Panic Recovery
```rust
// Automatic snapshot on panic
PanicHook::install(trace_system.get_sender());
```

### 4. Live Buffer Inspection
```rust
// For debugging/monitoring endpoints
let events: Vec<Event> = buffer.lock().unwrap().iter().cloned().collect();
serde_json::to_string(&events).unwrap()
```

## 📈 Performance Characteristics

- **Buffer Capacity**: 50,000 events (configurable)
- **Snapshot Frequency**: Triggered by business logic, not time
- **Compression**: LZ4 compression for efficient storage
- **Correlation**: Distributed tracing across service boundaries
- **Real-time**: Sub-millisecond event capture

## 🛠️ Production Considerations

1. **Buffer Sizing**: Adjust based on event volume and memory constraints
2. **Snapshot Cleanup**: Implement rotation/cleanup for old snapshots
3. **Correlation IDs**: Use consistent tracing across service boundaries
4. **Message Design**: Keep messages concise and self-contained
5. **Error Handling**: Graceful degradation when snapshot creation fails

## 🎉 What This Demonstrates

This example showcases `ttlog`'s capability to handle:
- **Enterprise-scale systems** with multiple services
- **Complex failure scenarios** with automatic recovery
- **Real-time observability** across distributed boundaries
- **Performance monitoring** with minimal overhead
- **Security auditing** with risk-based alerting
- **Business intelligence** through structured event logging

The system generates 50,000+ events across 12 services, demonstrating `ttlog`'s suitability for production distributed systems. 