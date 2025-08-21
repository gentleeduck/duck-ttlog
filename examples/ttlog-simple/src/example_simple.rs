use std::{sync::Arc, thread, time::Duration};

// Your logging library modules
use ttlog::{
  file_listener::FileListener,
  listener,
  trace::Trace,
  ttlog_macros::{error, info, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  // println!("TTLog Quick Start Example");

  // Step 1: Initialize the tracing system
  // Parameters: (ring_buffer_capacity, channel_capacity)
  // ttlog::stdout_listener::init_stdout();
  let trace = Trace::init(4096, 64, "default");
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));

  // Step 2: Use standard tracing macros to log
  info!("Application started successfully");
  warn!("This is a warning message");
  error!("This is an error message");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  info!(user_id = user_id, username = username, "User logged in");

  trace.request_snapshot("quick_start_example");

  info!("Application started successfully");
  warn!("This is a warning message");
  error!("This is an error message");
  // let result = std::panic::catch_unwind(|| {
  //   error!("Something went wrong before panic");
  //   panic!("Simulated application crash!");
  // });
  trace.request_snapshot("done");
  // panic!("Simulated application crash!");
  // Step 4: Request a manual snapshot (optional)
  // trace_system.request_snapshot("quick_start_example");

  // Step 5: Give time for the snapshot to be written
  // thread::sleep(Duration::from_millis(200));

  // println!("Done! Check /tmp/ for ttlog-*.bin files");
  // println!("Run: ls -la /tmp/ttlog-*.bin");

  Ok(())
}
