use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};
use ttlog::trace::Trace;

/// Example 7: Error scenarios and recovery
pub fn example_error_scenarios() {
  println!("\n=== Example 7: Error Scenarios ===");

  let trace_system = Trace::init(200, 20);

  // Simulate various error conditions
  warn!("Low memory detected");
  error!(
    error_type = "DatabaseError",
    retry_count = 1,
    "Failed to connect to database, retrying..."
  );

  info!("Retry successful");

  error!(
    error_type = "ValidationError",
    field = "email",
    value = "invalid-email",
    "Validation failed"
  );

  // Critical error - take immediate snapshot
  error!("Critical system error detected");
  trace_system.request_snapshot("critical_error");

  info!("System recovery initiated");
  info!("Recovery completed successfully");

  thread::sleep(Duration::from_millis(100));
}
