## Key Features This Example Demonstrates

### **High-Throughput Logging**
- Multiple concurrent services generating logs simultaneously
- Various log levels (TRACE, DEBUG, INFO, WARN, ERROR)
- Structured logging with key-value pairs using tracing macros

### **Real-World Scenarios**
- **API Gateway**: HTTP request processing with metrics
- **User Service**: Database operations with authentication flows
- **Order Service**: Complex business logic with error handling
- **Payment Service**: Deliberate failures and panic scenarios
- **Metrics Service**: System monitoring with alerting
- **Background Jobs**: Long-running tasks with varying execution times
- **Database Manager**: Connection pool management with scaling

### **Error Handling & Edge Cases**
- **Panic Recovery**: The payment service has a rare panic condition that triggers your panic hook
- **High Error Rates**: Payment service simulates realistic failure scenarios
- **Resource Scaling**: Database connection pool demonstrates dynamic scaling
- **Alert Conditions**: Metrics service shows threshold-based alerting

### **Performance Testing**
- **30-second high-load simulation** with 7 concurrent services
- **Thousands of log events** testing buffer wraparound
- **Structured data** with transaction IDs, user IDs, processing times
- **Pre-shutdown snapshot** to capture final state

## Running the Example

```bash
cargo run
```
