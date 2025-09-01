// examples/web_server.rs
//
// Real-world example: Integrating ttlog with a simple HTTP server

use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, instrument, warn};
use ttlog::{panic_hook::PanicHook, trace::Trace};

// Simulated HTTP request structure
#[derive(Debug)]
struct HttpRequest {
  method: String,
  path: String,
  user_id: Option<u32>,
  ip_address: String,
}

#[derive(Debug)]
struct HttpResponse {
  status_code: u16,
  response_time_ms: u64,
}

// Simulated web server
struct WebServer {
  trace_system: Trace,
  port: u16,
}

impl WebServer {
  fn new(port: u16) -> Self {
    // Initialize tracing with larger capacity for web server
    let trace_system = Trace::init(5000, 500);

    // Install panic hook for crash recovery
    PanicHook::install(trace_system.get_sender());

    info!(port = port, "Web server initializing");

    Self { trace_system, port }
  }

  fn start(&self) {
    info!(port = self.port, "Web server started");

    // Simulate handling requests
    for request_id in 1..=50 {
      let request = self.generate_mock_request(request_id);
      let response = self.handle_request(request_id, request);

      // Log response
      info!(
        request_id = request_id,
        status = response.status_code,
        response_time_ms = response.response_time_ms,
        "Request completed"
      );

      // Take snapshots at certain intervals
      if request_id % 20 == 0 {
        self
          .trace_system
          .request_snapshot(format!("batch_{}", request_id));
      }

      // Simulate request processing time
      thread::sleep(Duration::from_millis(10));
    }

    // Final snapshot when shutting down
    info!("Server shutting down");
    self.trace_system.request_snapshot("server_shutdown");
    thread::sleep(Duration::from_millis(200));
  }

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

    // Simulate different types of requests
    let response = match request.path.as_str() {
      "/api/users" => self.handle_users_endpoint(request_id, &request),
      "/api/auth" => self.handle_auth_endpoint(request_id, &request),
      "/api/health" => self.handle_health_endpoint(request_id),
      _ => self.handle_not_found(request_id, &request),
    };

    let response_time = start_time.elapsed().as_millis() as u64;

    HttpResponse {
      status_code: response,
      response_time_ms: response_time,
    }
  }

  fn handle_users_endpoint(&self, request_id: u32, request: &HttpRequest) -> u16 {
    if let Some(user_id) = request.user_id {
      info!(
        request_id = request_id,
        user_id = user_id,
        "Fetching user data"
      );

      // Simulate database lookup
      thread::sleep(Duration::from_millis(5));

      if user_id > 100 {
        warn!(request_id = request_id, user_id = user_id, "User not found");
        404
      } else {
        debug!(
          request_id = request_id,
          user_id = user_id,
          "User data retrieved successfully"
        );
        200
      }
    } else {
      warn!(request_id = request_id, "Missing user_id parameter");
      400
    }
  }

  fn handle_auth_endpoint(&self, request_id: u32, request: &HttpRequest) -> u16 {
    info!(request_id = request_id, "Processing authentication");

    // Simulate authentication logic
    thread::sleep(Duration::from_millis(8));

    // Randomly fail some auth requests for demo
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

  fn handle_health_endpoint(&self, request_id: u32) -> u16 {
    debug!(request_id = request_id, "Health check requested");
    200
  }

  fn handle_not_found(&self, request_id: u32, request: &HttpRequest) -> u16 {
    warn!(
        request_id = request_id,
        path = %request.path,
        "Endpoint not found"
    );
    404
  }

  fn generate_mock_request(&self, request_id: u32) -> HttpRequest {
    let paths = vec!["/api/users", "/api/auth", "/api/health", "/api/unknown"];
    let methods = vec!["GET", "POST", "PUT", "DELETE"];
    let ips = vec!["192.168.1.1", "10.0.0.1", "172.16.0.1", "203.0.113.1"];

    HttpRequest {
      method: methods[request_id as usize % methods.len()].to_string(),
      path: paths[request_id as usize % paths.len()].to_string(),
      user_id: if request_id % 3 == 0 {
        Some(request_id)
      } else {
        None
      },
      ip_address: ips[request_id as usize % ips.len()].to_string(),
    }
  }
}

// Background task simulation
fn background_worker(trace_system: &Trace) {
  thread::spawn(|| {
    for i in 1..=10 {
      info!(task_id = i, "Background task processing");

      // Simulate work
      thread::sleep(Duration::from_millis(50));

      if i % 3 == 0 {
        warn!(task_id = i, "Background task encountered warning");
      }

      debug!(task_id = i, "Background task completed");
    }

    info!("All background tasks completed");
  });
}

fn main() {
  println!("TTLog Web Server Example");
  println!("========================");

  // Create and start the web server
  let server = WebServer::new(8080);

  // Start background worker
  background_worker(&server.trace_system);

  // Start handling requests
  server.start();

  println!("\nServer simulation completed!");
  println!("Check /tmp/ for snapshot files:");
  println!("  ls -la /tmp/ttlog-*.bin");
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_web_server_creates_snapshots() {
    let server = WebServer::new(3000);

    // Generate a few mock requests
    for i in 1..=5 {
      let request = server.generate_mock_request(i);
      let _response = server.handle_request(i, request);
    }

    server.trace_system.request_snapshot("test_web_server");
    thread::sleep(Duration::from_millis(100));

    // Verify snapshot files were created
    let entries: Vec<_> = fs::read_dir("/tmp")
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| e.file_name().to_string_lossy().starts_with("ttlog-"))
      .collect();

    assert!(!entries.is_empty(), "Expected snapshot files to be created");
  }
}
