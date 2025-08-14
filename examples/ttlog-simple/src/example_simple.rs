use std::{thread, time::Duration};
use tracing::{debug, error, info, warn};

// Your logging library modules
use ttlog::{panic_hook::PanicHook, trace::Trace};

pub fn example_simple() {
  println!("TTLog Quick Start Example");

  // Step 1: Initialize the tracing system
  // Parameters: (ring_buffer_capacity, channel_capacity)
  let trace_system = Trace::init(1000, 100);

  // Step 2: Use standard tracing macros to log
  info!("Application started successfully");
  warn!("This is a warning message");
  error!("This is an error message");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  info!(user_id = user_id, username = username, "User logged in");

  // Step 4: Request a manual snapshot (optional)
  trace_system.request_snapshot("quick_start_example");

  // Step 5: Give time for the snapshot to be written
  thread::sleep(Duration::from_millis(200));

  println!("Done! Check /tmp/ for ttlog-*.bin files");
  println!("Run: ls -la /tmp/ttlog-*.bin");
}
