use std::thread;
use std::time::Duration;
use tracing::info;
use ttlog::trace::Trace;

/// Example 6: Custom service names and reasons
pub fn example_custom_service() {
  println!("\n=== Example 6: Custom Service Configuration ===");

  let trace_system = Trace::init(300, 30);

  // Log events for a specific service
  info!(service = "user-service", "User service starting");
  info!(service = "auth-service", "Authentication initialized");
  info!(service = "db-service", "Database connection established");

  // Take snapshots with descriptive reasons
  trace_system.request_snapshot("startup_complete");

  // Simulate some operations
  for i in 1..=10 {
    info!(
      service = "user-service",
      user_id = i,
      "Processing user {}",
      i
    );
  }

  trace_system.request_snapshot("user_batch_processed");
  thread::sleep(Duration::from_millis(150));
}
