I've created comprehensive examples showing how to use your ttlog library:

### 1. **Quick Start Example** (`quick_start.rs`)
- Minimal setup to get started
- Basic logging with tracing macros
- Manual snapshot requests

### 2. **Complete Usage Examples** (`basic_usage.rs`)
- **Basic logging**: Simple setup and logging
- **Structured logging**: Adding fields to log events
- **High-volume logging**: Stress testing with 5000+ events
- **Multi-threaded logging**: Concurrent logging from multiple threads
- **Panic handling**: Automatic snapshots on application crashes
- **Custom service names**: Organizing logs by service
- **Error scenarios**: Handling and logging errors

### 3. **Web Server Example** (`web_server.rs`)
- Real-world integration with a simulated HTTP server
- Request/response logging with structured data
- Background tasks logging
- Periodic snapshots during
