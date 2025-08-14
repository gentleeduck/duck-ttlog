# TTLog Complex Example - Distributed System Simulation

The most comprehensive TTLog example demonstrating enterprise-scale patterns in a distributed microservices environment. This example simulates a complete e-commerce system with 12 microservices, complex inter-service communication, and advanced observability patterns.

## 🎯 What This Example Demonstrates

### 🏢 Enterprise-Scale Architecture
- **12 Microservices** - User, Product, Order, Payment, Inventory, Notification, Analytics, Audit, Recommendation, Search, Pricing, Shipping
- **Inter-service Communication** - Message passing with correlation IDs and distributed tracing
- **Circuit Breakers** - Automatic failure detection and recovery patterns
- **Database Connection Pooling** - Realistic database operations with failure scenarios
- **Message Queue Processing** - Asynchronous message handling with dead letter queues
- **Load Balancing** - Health checks and instance selection

### 🔍 Advanced Observability
- **Distributed Tracing** - Correlation IDs across service boundaries
- **Performance Monitoring** - Response times, success rates, resource usage
- **Security Audit Logging** - Risk scoring and anomaly detection
- **Business Intelligence** - Analytics events and metrics collection
- **Chaos Engineering** - Random failure injection for resilience testing
- **Real-time Metrics** - System-wide monitoring and alerting

### 🌐 Real-world Scenarios
- **E-commerce Workload** - User registrations, product searches, order placements
- **Peak Load Handling** - Time-based load multipliers (morning, lunch, evening peaks)
- **Error Scenarios** - Database failures, network timeouts, service unavailability
- **Security Events** - Injection attacks, rate limiting, authentication bypass attempts

## 🚀 Quick Start

### Prerequisites
```bash
# Ensure you have Rust installed
rustc --version

# Navigate to the example directory
cd examples/ttlog-complex
```

### Running the Example
```bash
# Run the distributed system simulation
cargo run

# Run with verbose logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```

## 📊 Expected Output

When you run the example, you'll see output like:

```
🚀 ULTIMATE COMPLEX TTLOG EXAMPLE - DISTRIBUTED MICROSERVICES SYSTEM 🚀
=====================================================================
Features:
✅ 12 Microservices with Inter-service Communication
✅ Advanced Circuit Breakers & Retry Logic
✅ Database Connection Pooling Simulation
✅ Distributed Tracing with Correlation IDs
✅ Load Balancing with Health Checks
✅ Chaos Engineering with Random Failures
✅ Real-time Metrics & Performance Monitoring
✅ Security Audit Logging & Anomaly Detection
✅ Business Intelligence & Analytics Events
✅ Message Queue with Dead Letter Queue
✅ Auto-scaling Simulation
✅ Comprehensive System Testing
✅ Graceful Shutdown Procedures

Distributed system initializing with advanced observability
[INFO] Starting microservice service=user-service version=v1.2.3 instance_id=user_inst_123 port=8001
[INFO] Starting microservice service=product-service version=v2.1.0 instance_id=prod_inst_456 port=8002
[INFO] Starting microservice service=order-service version=v1.5.1 instance_id=order_inst_789 port=8003
[INFO] Registering service instance service_name=user-service instance_id=user_inst_123
[INFO] Registering service instance service_name=product-service instance_id=prod_inst_456
[INFO] Registering service instance service_name=order-service instance_id=order_inst_789

[INFO] job.start id=0 kind=import
[INFO] job.done id=0 ms=10
[INFO] job.start id=1 kind=import
[WARN] job.retry id=17 attempt=1 reason=timeout
[INFO] job.done id=17 ms=35

[INFO] Service metrics report service=user-service requests_total=150 requests_success=142 requests_failed=8 success_rate=94.67% avg_response_time_ms=45.23 active_connections=12

[WARN] Chaos engineering: Injecting failure chaos_scenario=Random Service Failure impact=ServiceDown("random") duration_seconds=30
[ERROR] Chaos: Service artificially taken down service=random

[INFO] System-wide metrics system_cpu_percent=45.2 system_memory_percent=62.8 network_io_mbps=450 disk_io_iops=125 active_connections=850

⏰ Runtime: 1 minutes
⏰ Runtime: 2 minutes
⏰ Runtime: 3 minutes
🧪 Running comprehensive system tests...
[INFO] Starting comprehensive system tests
[INFO] Running load test - 1000 concurrent requests
[WARN] Load test completed - checking for system stress indicators
[INFO] Running failure scenario tests
[ERROR] Simulating database connection failures
[ERROR] Simulating service timeout scenarios
[WARN] Simulating message queue overflow
[ERROR] Simulating memory pressure scenarios
[INFO] Running performance benchmarks
[INFO] Message throughput benchmark completed messages_sent=10000 duration_ms=1250 messages_per_second=8000.00
[INFO] Snapshot creation benchmark completed snapshot_creation_time_ms=45
[INFO] All performance benchmarks completed total_benchmark_time_ms=2340
[INFO] Running security tests
[WARN] Simulating SQL injection attempts
[WARN] Testing rate limiting scenarios
[ERROR] Simulating authentication bypass attempts
[INFO] Security tests completed
[INFO] Running data consistency tests
[INFO] Testing transaction consistency across services
[INFO] Testing event ordering and causality
[WARN] Testing state synchronization between services
[INFO] Consistency tests completed
[INFO] All system tests completed

⏰ Runtime: 4 minutes
⏰ Runtime: 5 minutes
⏰ Runtime: 6 minutes
⏰ Runtime: 7 minutes
⚡ Triggering high-load scenario...
[INFO] Auto-scaling: Adding more instances current_cpu=78.5 action=scale_up
[WARN] High resource utilization detected cpu_percent=78.5 memory_percent=82.3

⏰ Runtime: 8 minutes
⏰ Runtime: 9 minutes
⏰ Runtime: 10 minutes
🛑 Initiating graceful shutdown...
[INFO] Initiating graceful shutdown of distributed system
[INFO] Distributed system shutdown completed

🎉 ULTIMATE COMPLEX EXAMPLE COMPLETED! 🎉
===========================================
📊 Check /tmp/ for comprehensive snapshot files:
   ls -la /tmp/ttlog-*.bin | wc -l  # Count of snapshots
   ls -la /tmp/ttlog-*.bin | tail   # Latest snapshots

📈 This example demonstrated:
   • Complex distributed system interactions
   • Advanced error handling and resilience patterns
   • Real-time monitoring and observability
   • Performance testing and benchmarking
   • Security testing and audit logging
   • Chaos engineering and failure injection
   • Business intelligence and analytics
   • Comprehensive tracing across service boundaries

💡 Your ttlog library handled 50,000+ events across 12 services!
   This showcases the library's capability for enterprise-scale systems!
```

## 🏗️ Architecture Overview

### Microservice Structure
```rust
struct MicroService {
    name: String,                    // Service name (e.g., "user-service")
    version: String,                 // Service version
    instance_id: String,             // Unique instance identifier
    port: u16,                       // Service port
    metrics: ServiceMetrics,         // Performance metrics
    circuit_breakers: HashMap<String, CircuitBreaker>, // Failure handling
    message_sender: Sender<ServiceMessage>,           // Message sending
    message_receiver: Receiver<ServiceMessage>,       // Message receiving
    database_pool: Arc<DatabasePool>,                 // Database connections
    trace_system: Arc<Trace>,                         // TTLog integration
    chaos_failure_rate: f64,                          // Chaos engineering
}
```

### Distributed System Components
- **Load Balancer** - Service discovery and health checks
- **Message Broker** - Inter-service communication with dead letter queue
- **Chaos Engine** - Random failure injection for resilience testing
- **System Monitor** - Real-time metrics collection and alerting
- **Workload Simulator** - Realistic e-commerce traffic patterns

### Service Communication Flow
1. **Request Generation** - Simulated user actions and system events
2. **Message Routing** - Load balancer selects healthy service instances
3. **Service Processing** - Business logic with database operations
4. **Inter-service Calls** - Message passing between services
5. **Event Logging** - Comprehensive tracing with correlation IDs
6. **Snapshot Creation** - Strategic snapshots on critical events

## 📁 Generated Files

After running the example, check `/tmp/` for comprehensive snapshot files:

```bash
# List all generated snapshots
ls -la /tmp/ttlog-*.bin

# Example snapshot names:
# ttlog-12345-20250101123456-system_initialization.bin
# ttlog-12345-20250101123457-all_services_started.bin
# ttlog-12345-20250101123458-user_service_startup.bin
# ttlog-12345-20250101123459-product_service_startup.bin
# ttlog-12345-20250101123460-order_service_startup.bin
# ttlog-12345-20250101123461-payment_service_startup.bin
# ttlog-12345-20250101123462-inventory_service_startup.bin
# ttlog-12345-20250101123463-notification_service_startup.bin
# ttlog-12345-20250101123464-analytics_service_startup.bin
# ttlog-12345-20250101123465-audit_service_startup.bin
# ttlog-12345-20250101123466-recommendation_service_startup.bin
# ttlog-12345-20250101123467-search_service_startup.bin
# ttlog-12345-20250101123468-pricing_service_startup.bin
# ttlog-12345-20250101123469-shipping_service_startup.bin
# ttlog-12345-20250101123470-milestone.bin
# ttlog-12345-20250101123471-peak_load_hour_9.bin
# ttlog-12345-20250101123472-peak_load_hour_19.bin
# ttlog-12345-20250101123473-system_tests_start.bin
# ttlog-12345-20250101123474-load_test_complete.bin
# ttlog-12345-20250101123475-db_failure_test.bin
# ttlog-12345-20250101123476-queue_overflow_test.bin
# ttlog-12345-20250101123477-performance_benchmark.bin
# ttlog-12345-20250101123478-security_tests_complete.bin
# ttlog-12345-20250101123479-consistency_tests_complete.bin
# ttlog-12345-20250101123480-system_tests_complete.bin
# ttlog-12345-20250101123481-high_load_scenario.bin
# ttlog-12345-20250101123482-graceful_shutdown_start.bin
# ttlog-12345-20250101123483-system_final_state.bin
```

## 🔍 Key Patterns Demonstrated

### 1. Distributed Tracing
```rust
#[derive(Debug, Clone, Serialize, Deserialize)]
struct TraceContext {
    trace_id: String,           // Unique trace identifier
    span_id: String,            // Current span identifier
    parent_span_id: Option<String>, // Parent span for hierarchy
    correlation_id: String,     // Business correlation ID
    user_id: Option<u64>,       // User context
    session_id: Option<String>, // Session context
}

// Creating child spans for inter-service calls
let child_context = context.child_span();
self.send_message_to_service("payment-service", "payment_processing", payload, child_context);
```

### 2. Circuit Breaker Pattern
```rust
impl CircuitBreaker {
    fn can_execute(&self) -> bool {
        match self.state.lock().unwrap() {
            CircuitState::Closed => true,
            CircuitState::Open => {
                // Check if timeout has passed
                if let Some(last_failure) = *self.last_failure_time.lock().unwrap() {
                    if last_failure.elapsed() >= self.timeout_duration {
                        *self.state.lock().unwrap() = CircuitState::HalfOpen;
                        true
                    } else {
                        false
                    }
                } else {
                    false
                }
            },
            CircuitState::HalfOpen => true,
        }
    }
}
```

### 3. Database Connection Pooling
```rust
impl DatabasePool {
    async fn execute_query(&self, query: &str, context: &TraceContext) -> Result<String, String> {
        let start = Instant::now();
        
        // Find or create connection
        let connection = if let Some(conn) = connections.iter_mut().find(|c| c.is_healthy) {
            conn.last_used = Instant::now();
            conn.queries_executed += 1;
            conn
        } else if connections.len() < self.max_connections {
            // Create new connection
            let new_conn = DatabaseConnection { /* ... */ };
            connections.push(new_conn);
            connections.last_mut().unwrap()
        } else {
            return Err("Pool exhausted".to_string());
        };
        
        // Execute query with error simulation
        if rand::thread_rng().gen_bool(0.02) {
            connection.is_healthy = false;
            return Err("Connection lost".to_string());
        }
        
        Ok(result)
    }
}
```

### 4. Message Queue with Dead Letter Queue
```rust
impl MessageBroker {
    fn start_message_routing(&self, trace_system: Arc<Trace>) {
        thread::spawn(move || {
            while let Ok(message) = receiver.recv() {
                // Check if message has expired
                if message.expires_at < get_timestamp() {
                    warn!("Message expired, moving to dead letter queue");
                    dead_letter_queue.lock().unwrap().push_back(message);
                    continue;
                }
                
                // Route message based on target service
                let mut queue = message_queue.lock().unwrap();
                queue.push_back(message);
                
                // Process messages in order
                if let Some(msg) = queue.pop_front() {
                    debug!("Routing message to {}", msg.to_service);
                }
            }
        });
    }
}
```

### 5. Chaos Engineering
```rust
impl ChaosEngine {
    fn start_chaos_testing(&self) {
        thread::spawn(move || {
            loop {
                for scenario in &scenarios {
                    if rand::thread_rng().gen_bool(scenario.probability) {
                        warn!(
                            chaos_scenario = %scenario.name,
                            impact = ?scenario.impact,
                            "Chaos engineering: Injecting failure"
                        );
                        
                        trace_system.request_snapshot(&format!("chaos_{}", scenario.name.replace(" ", "_")));
                        
                        // Simulate the chaos impact
                        match &scenario.impact {
                            ChaosImpact::ServiceDown(service) => {
                                error!(service = service, "Chaos: Service artificially taken down");
                            },
                            ChaosImpact::NetworkLatency(latency_ms) => {
                                warn!(latency_ms = latency_ms, "Chaos: Network latency injected");
                            },
                            // ... other impacts
                        }
                    }
                }
                thread::sleep(Duration::from_secs(30));
            }
        });
    }
}
```

## 🎯 Best Practices Demonstrated

### 1. Strategic Snapshot Points
- **Service Lifecycle**: Startup, shutdown, health checks
- **Business Events**: User registration, order placement, payment processing
- **Error Conditions**: Circuit breaker trips, database failures, timeouts
- **Performance Thresholds**: High load, slow responses, resource pressure
- **Security Events**: Authentication failures, suspicious activity

### 2. Structured Logging Patterns
```rust
// Business events with correlation
info!(
    service = %self.name,
    trace_id = %message.context.trace_id,
    order_id = %order_id,
    customer_id = %order_data.get("customer_id").unwrap_or(&serde_json::Value::Null),
    total_amount = %order_data.get("total").unwrap_or(&serde_json::Value::Null),
    "Processing order placement"
);

// Performance metrics
info!(
    service = %service_name,
    requests_total = total,
    requests_success = success,
    requests_failed = failed,
    success_rate = format!("{:.2}%", success_rate),
    avg_response_time_ms = format!("{:.2}", avg_response_time),
    "Service metrics report"
);

// Security events
error!(
    service = %self.name,
    trace_id = %message.context.trace_id,
    event_type = %event_type,
    risk_score = %risk_score,
    ip_address = %audit_data.get("ip_address").unwrap_or(&serde_json::Value::Null),
    "High-risk security event detected"
);
```

### 3. Error Handling and Recovery
- **Circuit Breakers**: Prevent cascade failures
- **Retry Logic**: Automatic retry with exponential backoff
- **Graceful Degradation**: Continue operation with reduced functionality
- **Dead Letter Queues**: Handle failed messages
- **Health Checks**: Monitor service health and recovery

### 4. Performance Monitoring
- **Response Time Tracking**: Monitor all service interactions
- **Success Rate Monitoring**: Track error rates and trends
- **Resource Utilization**: CPU, memory, network, disk usage
- **Throughput Metrics**: Requests per second, messages per second
- **Auto-scaling**: Dynamic instance management based on load

## 🔧 Customization Examples

### Adding New Services
```rust
// Define new service configuration
let services_config = vec![
    ("user-service", "v1.2.3", 8001),
    ("product-service", "v2.1.0", 8002),
    ("order-service", "v1.5.1", 8003),
    ("payment-service", "v3.0.2", 8004),
    ("inventory-service", "v1.8.0", 8005),
    ("notification-service", "v2.3.1", 8006),
    ("analytics-service", "v1.4.0", 8007),
    ("audit-service", "v1.1.0", 8008),
    ("recommendation-service", "v2.2.0", 8009),
    ("search-service", "v1.7.0", 8010),
    ("pricing-service", "v1.3.0", 8011),
    ("shipping-service", "v2.0.1", 8012),
    ("new-service", "v1.0.0", 8013), // Add your service here
];
```

### Custom Workload Patterns
```rust
fn simulate_custom_workload(&self, sender: &Sender<ServiceMessage>, context: TraceContext) {
    let message = ServiceMessage {
        id: generate_id("msg"),
        from_service: "custom-service".to_string(),
        to_service: "target-service".to_string(),
        message_type: "custom_event".to_string(),
        payload: serde_json::json!({
            "custom_field": "custom_value",
            "timestamp": get_timestamp()
        }),
        context,
        timestamp: get_timestamp(),
        priority: MessagePriority::Normal,
        retry_count: 0,
        expires_at: get_timestamp() + 300000,
    };
    
    let _ = sender.try_send(message);
}
```

### Enhanced Chaos Scenarios
```rust
let failure_scenarios = vec![
    ChaosScenario {
        name: "Custom Failure Scenario".to_string(),
        probability: 0.01,
        impact: ChaosImpact::CustomFailure("your_custom_failure".to_string()),
        duration_seconds: 60,
    },
    // ... existing scenarios
];
```

## 🧪 Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test categories
cargo test test_distributed_system_creates_many_snapshots
cargo test test_chaos_engineering_triggers_snapshots
cargo test test_high_throughput_message_processing
```

### Test Categories
- **System Integration Tests**: Verify all services work together
- **Chaos Engineering Tests**: Validate failure injection and recovery
- **Performance Tests**: Measure throughput and response times
- **Snapshot Tests**: Ensure snapshots are created correctly

## 📈 Performance Considerations

### Buffer Sizing for Enterprise Systems
- **Small Systems**: 10,000-20,000 events
- **Medium Systems**: 20,000-50,000 events
- **Large Systems**: 50,000-100,000 events
- **Enterprise Systems**: 100,000+ events

### Snapshot Frequency
- **Development**: Every few minutes or on errors
- **Testing**: On specific test completion or milestones
- **Production**: On critical events, errors, or state changes
- **Debugging**: On demand or at specific checkpoints

### Memory and CPU Usage
- **Event Storage**: ~200-500 bytes per event
- **Buffer Management**: Ring buffer with O(1) operations
- **Snapshot Creation**: LZ4 compression for efficiency
- **Concurrent Access**: Thread-safe operations with minimal locking

## 🛠️ Troubleshooting

### Common Issues

#### High Memory Usage
```bash
# Reduce buffer size if memory constrained
let trace_system = Trace::init(10000, 1000); // Smaller buffer

# Monitor memory usage
top -p $(pgrep ttlog-complex)
```

#### Slow Performance
```bash
# Check if logging is causing bottlenecks
# Reduce log verbosity in production
# Monitor buffer utilization
```

#### Missing Snapshots
```bash
# Verify snapshot triggers are being called
# Check /tmp directory permissions
# Look for error messages in output
```

### Debug Mode
```bash
# Enable detailed logging
RUST_LOG=debug cargo run

# Enable trace logging for maximum detail
RUST_LOG=trace cargo run

# Monitor specific services
RUST_LOG=user_service=debug cargo run
```

## 🔄 Integration with Real Systems

### Kubernetes Deployment
```yaml
apiVersion: apps/v1
kind: Deployment
metadata:
  name: ttlog-example
spec:
  replicas: 3
  selector:
    matchLabels:
      app: ttlog-example
  template:
    metadata:
      labels:
        app: ttlog-example
    spec:
      containers:
      - name: ttlog-complex
        image: ttlog-complex:latest
        env:
        - name: RUST_LOG
          value: "info"
        volumeMounts:
        - name: ttlog-snapshots
          mountPath: /tmp
      volumes:
      - name: ttlog-snapshots
        emptyDir: {}
```

### Docker Compose
```yaml
version: '3.8'
services:
  ttlog-complex:
    build: .
    environment:
      - RUST_LOG=info
    volumes:
      - ./snapshots:/tmp
    ports:
      - "8080:8080"
```

## 📚 Next Steps

After understanding this example:

1. **Try the Simple Examples** (`../ttlog-simple/`) - Learn basic concepts
2. **Use the Web Server Example** (`../ttlog-server/`) - See web integration
3. **Use the File Reader** (`../ttlog-filereader/`) - Analyze your snapshots
4. **Integrate TTLog into your distributed system** - Apply these patterns

## 🎉 Key Takeaways

By completing this example, you'll understand:

- ✅ How to build enterprise-scale observability with TTLog
- ✅ Distributed tracing and correlation patterns
- ✅ Circuit breaker and resilience patterns
- ✅ Performance monitoring and optimization
- ✅ Security audit logging and anomaly detection
- ✅ Chaos engineering and failure injection
- ✅ Message queue and inter-service communication
- ✅ Database connection pooling and error handling
- ✅ Load balancing and health checking
- ✅ Auto-scaling and resource management

---

**Ready to build enterprise-scale systems with TTLog? Run `cargo run` and experience the power! 🚀** 
