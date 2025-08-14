use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};
use ttlog::trace::Trace;

/// Example 2: Logging with structured data
pub fn example_structured_logging() {
  println!("\n=== Example 2: Structured Logging ===");

  let _trace_system = Trace::init(500, 50);

  // Log with structured fields
  let user_id = 12345;
  let username = "alice";

  info!(user_id = user_id, username = username, "User logged in");
  warn!(error_code = 404, path = "/api/users", "Page not found");
  error!(
    exception = "NullPointerException",
    line = 42,
    file = "main.rs",
    "Critical error occurred"
  );

  thread::sleep(Duration::from_millis(100));
}
