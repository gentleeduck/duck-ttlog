# TTLog Web Server Example

A real-world example demonstrating how to integrate TTLog into a web server application. This example shows practical patterns for logging HTTP requests, handling authentication, monitoring performance, and managing background tasks in a web service context.

## 🎯 What This Example Demonstrates

### 🌐 Web Server Integration
- **HTTP Request/Response Logging** - Complete request lifecycle tracking
- **Authentication & Authorization** - Security event logging
- **Performance Monitoring** - Response time tracking and analysis
- **Error Handling** - Graceful error logging and recovery
- **Background Task Management** - Concurrent task logging
- **Request Correlation** - Linking related events across the system

### 🔧 Key Features
- Simulated HTTP server with multiple endpoints
- Realistic request generation and processing
- Performance metrics collection
- Error scenario simulation
- Background worker integration
- Graceful shutdown handling

## 🚀 Quick Start

### Prerequisites
```bash
# Ensure you have Rust installed
rustc --version

# Navigate to the example directory
cd examples/ttlog-server
```

### Running the Example
```bash
# Run the web server simulation
cargo run

# Run with verbose logging
RUST_LOG=debug cargo run

# Run tests
cargo test
```

## 📊 Expected Output

When you run the example, you'll see output like:

```
TTLog Web Server Example
========================

[INFO] Web server initializing port=8080
[INFO] Web server started port=8080
[INFO] Background task processing task_id=1
[DEBUG] Processing request request_id=1 method=GET path=/api/users ip=192.168.1.1
[INFO] Fetching user data request_id=1 user_id=1
[DEBUG] User data retrieved successfully request_id=1 user_id=1
[INFO] Request completed request_id=1 status=200 response_time_ms=15
[INFO] Processing authentication request_id=2
[INFO] Authentication successful request_id=2
[INFO] Request completed request_id=2 status=200 response_time_ms=18
[WARN] Background task encountered warning task_id=3
[ERROR] Authentication failed - invalid credentials request_id=7 ip=10.0.0.1
[INFO] Request completed request_id=7 status=401 response_time_ms=12
[INFO] All background tasks completed
[INFO] Server shutting down

Server simulation completed!
Check /tmp/ for snapshot files:
  ls -la /tmp/ttlog-*.bin
```

## 🏗️ Architecture Overview

### WebServer Structure
```rust
struct WebServer {
    trace_system: Trace,  // TTLog tracing system
    port: u16,           // Server port
}
```

### Request Flow
1. **Request Generation** - Simulated HTTP requests with various endpoints
2. **Request Processing** - Route-based handling with logging
3. **Response Generation** - Status codes and timing information
4. **Event Logging** - Complete request lifecycle tracking
5. **Snapshot Creation** - Periodic snapshots for analysis

### Endpoints Simulated
- `/api/users` - User data retrieval
- `/api/auth` - Authentication processing
- `/api/health` - Health check endpoint
- `/api/unknown` - 404 error handling

## 📁 Generated Files

After running the example, check `/tmp/` for snapshot files:

```bash
# List generated snapshots
ls -la /tmp/ttlog-*.bin

# Example snapshot names:
# ttlog-12345-20250101123456-batch_20.bin
# ttlog-12345-20250101123457-batch_40.bin
# ttlog-12345-20250101123458-server_shutdown.bin
```

## 🔍 Key Logging Patterns

### Request Logging
```rust
#[instrument(skip(self))]
fn handle_request(&self, request_id: u32, request: HttpRequest) -> HttpResponse {
    let start_time = std::time::Instant::now();
    
    debug!(
        request_id = request_id,
        method = %request.method,
        path = %request.path,
        ip = %request.ip_address,
        "Processing request"
    );
    
    // ... request processing ...
    
    let response_time = start_time.elapsed().as_millis() as u64;
    
    info!(
        request_id = request_id,
        status = response.status_code,
        response_time_ms = response.response_time_ms,
        "Request completed"
    );
}
```

### Authentication Logging
```rust
fn handle_auth_endpoint(&self, request_id: u32, request: &HttpRequest) -> u16 {
    info!(request_id = request_id, "Processing authentication");
    
    // Simulate authentication logic
    if request_id % 7 == 0 {
        error!(
            request_id = request_id,
            ip = %request.ip_address,
            "Authentication failed - invalid credentials"
        );
        401
    } else {
        info!(request_id = request_id, "Authentication successful");
        200
    }
}
```

### Background Task Logging
```rust
fn background_worker(trace_system: &Trace) {
    thread::spawn(|| {
        for i in 1..=10 {
            info!(task_id = i, "Background task processing");
            
            if i % 3 == 0 {
                warn!(task_id = i, "Background task encountered warning");
            }
            
            debug!(task_id = i, "Background task completed");
        }
    });
}
```

## 🎯 Best Practices Demonstrated

### 1. Structured Logging
- Use consistent field names (`request_id`, `user_id`, `response_time_ms`)
- Include relevant context in every log message
- Use appropriate log levels (debug, info, warn, error)

### 2. Performance Monitoring
- Track response times for all requests
- Log performance metrics consistently
- Use structured data for easy analysis

### 3. Error Handling
- Log errors with sufficient context
- Include request identifiers in error messages
- Use appropriate error status codes

### 4. Request Correlation
- Include `request_id` in all related log messages
- Maintain context across different functions
- Enable easy request tracing

### 5. Snapshot Strategy
- Take snapshots at logical intervals (every 20 requests)
- Create snapshots on shutdown
- Use descriptive snapshot reasons

## 🔧 Customization Examples

### Adding New Endpoints
```rust
fn handle_new_endpoint(&self, request_id: u32, request: &HttpRequest) -> u16 {
    info!(
        request_id = request_id,
        endpoint = "new_endpoint",
        "Processing new endpoint"
    );
    
    // Your endpoint logic here
    
    info!(request_id = request_id, "New endpoint completed");
    200
}
```

### Custom Request Generation
```rust
fn generate_custom_request(&self, request_id: u32) -> HttpRequest {
    HttpRequest {
        method: "POST".to_string(),
        path: "/api/custom".to_string(),
        user_id: Some(request_id),
        ip_address: "127.0.0.1".to_string(),
    }
}
```

### Enhanced Performance Logging
```rust
fn log_performance_metrics(&self, request_id: u32, response_time: u64) {
    if response_time > 100 {
        warn!(
            request_id = request_id,
            response_time_ms = response_time,
            "Slow request detected"
        );
    }
    
    info!(
        request_id = request_id,
        response_time_ms = response_time,
        "Request performance logged"
    );
}
```

## 🧪 Testing

### Running Tests
```bash
# Run all tests
cargo test

# Run specific test
cargo test test_web_server_creates_snapshots
```

### Test Structure
```rust
#[test]
fn test_web_server_creates_snapshots() {
    let server = WebServer::new(3000);
    
    // Generate mock requests
    for i in 1..=5 {
        let request = server.generate_mock_request(i);
        let _response = server.handle_request(i, request);
    }
    
    server.trace_system.request_snapshot("test_web_server");
    
    // Verify snapshots were created
    let entries: Vec<_> = std::fs::read_dir("/tmp")
        .unwrap()
        .filter_map(|e| e.ok())
        .filter(|e| e.file_name().to_string_lossy().starts_with("ttlog-"))
        .collect();
        
    assert!(!entries.is_empty());
}
```

## 📈 Performance Considerations

### Buffer Sizing for Web Servers
- **Development**: 1,000-2,000 events
- **Testing**: 5,000-10,000 events
- **Production**: 10,000-50,000 events (based on request volume)

### Snapshot Frequency
- **Development**: Every 10-20 requests
- **Testing**: Every 50-100 requests
- **Production**: Every 1,000+ requests or on errors

### Memory Usage
- Each HTTP request generates 3-5 log events
- Monitor buffer utilization in high-traffic scenarios
- Adjust buffer size based on expected request volume

## 🛠️ Troubleshooting

### Common Issues

#### No Requests Being Logged
```bash
# Check if request generation is working
# Verify tracing macros are being called
# Ensure TTLog is properly initialized
```

#### High Response Times
```bash
# Check if logging is causing performance issues
# Consider reducing log verbosity in production
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
```

## 🔄 Integration with Real Web Frameworks

### With Axum
```rust
use axum::{routing::get, Router};
use ttlog::{panic_hook::PanicHook, trace::Trace};

#[tokio::main]
async fn main() {
    let trace_system = Trace::init(10000, 1000);
    PanicHook::install(trace_system.get_sender());
    
    let app = Router::new()
        .route("/api/users", get(handle_users))
        .route("/api/auth", get(handle_auth));
        
    axum::Server::bind(&"0.0.0.0:3000".parse().unwrap())
        .serve(app.into_make_service())
        .await
        .unwrap();
}

async fn handle_users() -> String {
    info!("Handling users request");
    "Users endpoint".to_string()
}
```

### With Actix-web
```rust
use actix_web::{web, App, HttpServer};
use ttlog::{panic_hook::PanicHook, trace::Trace};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    let trace_system = Trace::init(10000, 1000);
    PanicHook::install(trace_system.get_sender());
    
    HttpServer::new(|| {
        App::new()
            .route("/api/users", web::get().to(handle_users))
            .route("/api/auth", web::get().to(handle_auth))
    })
    .bind("127.0.0.1:8080")?
    .run()
    .await
}
```

## 📚 Next Steps

After understanding this example:

1. **Try the Simple Examples** (`../ttlog-simple/`) - Learn basic concepts
2. **Use the File Reader** (`../ttlog-filereader/`) - Analyze your snapshots
3. **Explore the Complex Example** (`../ttlog-complex/`) - Enterprise patterns
4. **Integrate TTLog into your web application** - Apply these patterns

## 🎉 Key Takeaways

By completing this example, you'll understand:

- ✅ How to integrate TTLog into web applications
- ✅ Best practices for HTTP request logging
- ✅ Performance monitoring and optimization
- ✅ Error handling in web contexts
- ✅ Background task management
- ✅ Request correlation and tracing
- ✅ Production-ready logging patterns

---

**Ready to build robust web applications with TTLog? Run `cargo run` and see it in action! 🚀** 