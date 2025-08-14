use std::thread;
use std::time::Duration;
use tracing::{debug, info, warn};
use ttlog::trace::Trace;

/// Example 3: High-volume logging (stress test)
pub fn example_high_volume_logging() {
  println!("\n=== Example 3: High Volume Logging ===");

  let trace_system = Trace::init(2000, 200);

  // Simulate high-volume logging
  for i in 0..5000 {
    if i % 1000 == 0 {
      info!("Processed {} items", i);
    }
    debug!("Processing item {}", i);

    if i % 2000 == 0 && i > 0 {
      // Take snapshot every 2000 items
      trace_system.request_snapshot(&format!("batch_{}", i));
    }
  }

  warn!("High volume logging completed");
  thread::sleep(Duration::from_millis(300));
}
