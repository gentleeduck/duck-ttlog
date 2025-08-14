use std::{thread, time::Duration};
use tracing::{debug, error, info, warn};

// Your logging library modules
use ttlog::{panic_hook::PanicHook, trace::Trace};

fn main() {
  // Initialize the logging system
  // capacity: ring buffer size (how many events to keep in memory)
  // channel_capacity: how many events can be queued between threads
  let trace = Trace::init(10, 5_000);

  // Install panic hook to capture crashes
  PanicHook::install(trace.get_sender());

  println!("🚀 Logging system initialized!");

  // Now you can use standard tracing macros anywhere in your application
  info!("Application started successfully");
  warn!("This is a warning message");
  error!("This is an error message");
  debug!("Debug information");
  panic!("Trigger panic for testing hook");

  // Example: Simulate application work with logging
  simulate_application_work();

  // Example: Simulate some concurrent work
  simulate_concurrent_work();

  // Manually request a snapshot (optional)
  trace.request_snapshot("manual_checkpoint");

  // Give the writer thread time to process
  thread::sleep(Duration::from_millis(200));

  info!("Application finished");

  // The system automatically:
  // 1. Captures all log events in a ring buffer
  // 2. Creates periodic snapshots every 60 seconds
  // 3. Creates snapshots on panic
  // 4. Compresses and stores snapshots in /tmp/
}

fn simulate_application_work() {
  info!("Starting work simulation...");

  for i in 1..=100 {
    if i % 20 == 0 {
      warn!("Progress update: {}/100 items processed", i);
    } else {
      debug!("Processing item {}", i);
    }

    // Simulate some work
    thread::sleep(Duration::from_millis(10));

    if i == 75 {
      error!("Encountered an error at item {}, but continuing", i);
    }
  }

  info!("Work simulation completed");
}

fn simulate_concurrent_work() {
  info!("Starting concurrent logging test...");

  let handles: Vec<_> = (0..5)
    .map(|worker_id| {
      thread::spawn(move || {
        for task in 0..50 {
          info!("Worker {} completed task {}", worker_id, task);

          if task % 10 == 0 {
            warn!("Worker {} checkpoint at task {}", worker_id, task);
          }

          thread::sleep(Duration::from_millis(5));
        }
        info!("Worker {} finished all tasks", worker_id);
      })
    })
    .collect();

  // Wait for all workers
  for handle in handles {
    handle.join().unwrap();
  }

  info!("All concurrent work completed");
}

// Example of structured logging with context
fn example_with_context() {
  let user_id = 12345;
  let request_id = "req_abc123";

  // You can add context to your log messages
  info!("Processing request {} for user {}", request_id, user_id);

  match process_user_request(user_id) {
    Ok(result) => {
      info!(
        "Request {} completed successfully: {:?}",
        request_id, result
      );
    },
    Err(e) => {
      error!("Request {} failed for user {}: {}", request_id, user_id, e);
    },
  }
}

fn process_user_request(user_id: u32) -> Result<String, &'static str> {
  if user_id == 0 {
    Err("Invalid user ID")
  } else {
    Ok(format!("User {} data processed", user_id))
  }
}

// Example of panic handling
fn example_panic_scenario() {
  info!("About to trigger a panic for demonstration");

  // Add some context before panic
  warn!("System state: memory usage high, processing critical task");
  error!("Critical error detected, system unstable");

  // This will trigger the panic hook and create a snapshot
  // panic!("Simulated critical system failure");

  // Instead of actual panic, let's simulate recovery
  error!("Critical error handled, system recovered");
}

#[cfg(feature = "example_integration")]
fn integration_example() {
  use serde_json::json;

  // Example of how this might integrate with a web server
  async fn handle_request(req_id: &str) {
    info!("Handling HTTP request {}", req_id);

    match process_business_logic().await {
      Ok(data) => {
        info!("Request {} processed successfully", req_id);
        // Response data would be returned here
      },
      Err(e) => {
        error!("Request {} failed: {}", req_id, e);
        // Error response would be returned here
      },
    }
  }

  async fn process_business_logic() -> Result<serde_json::Value, String> {
    debug!("Starting business logic processing");

    // Simulate database call
    tokio::time::sleep(Duration::from_millis(50)).await;
    debug!("Database query completed");

    // Simulate some validation
    warn!("Validation warning: deprecated field used");

    Ok(json!({"status": "success", "data": "processed"}))
  }
}

// Configuration example
#[derive(Clone)]
struct AppConfig {
  log_buffer_size: usize,
  log_channel_size: usize,
  service_name: String,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      log_buffer_size: 10_000, // Keep last 10k events
      log_channel_size: 5_000, // Channel buffer
      service_name: "my_app".to_string(),
    }
  }
}

fn init_with_config(config: AppConfig) -> Trace {
  let trace = Trace::init(config.log_buffer_size, config.log_channel_size);

  // Install panic handling
  PanicHook::install(trace.get_sender());

  info!("Initialized logging for service: {}", config.service_name);

  trace
}

// Performance monitoring example
fn performance_monitoring_example() {
  let start = std::time::Instant::now();

  info!("Starting performance critical operation");

  // Simulate work
  for i in 0..1000 {
    if i % 100 == 0 {
      let elapsed = start.elapsed();
      info!("Performance checkpoint {}: {:?} elapsed", i, elapsed);
    }

    // Simulate CPU intensive work
    let _: Vec<u64> = (0..1000).map(|x| x * x).collect();
  }

  let total_time = start.elapsed();
  info!(
    "Performance critical operation completed in {:?}",
    total_time
  );

  if total_time > Duration::from_millis(500) {
    warn!("Operation took longer than expected: {:?}", total_time);
  }
}
