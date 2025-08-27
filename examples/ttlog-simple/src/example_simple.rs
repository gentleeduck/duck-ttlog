use std::sync::Arc;

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{debug, error, fatal, info, trace, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  println!("TTLog Quick Start Example");

  // Step 1: Initialize the tracing system
  let trace = Trace::init(4096, 64, "test", Some("./tmp"));

  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
  std::thread::sleep(std::time::Duration::from_millis(10));

  // Step 2: Use standard tracing macros to log
  trace!("Application started successfully");
  debug!("Application started successfully");
  info!("Application started successfully");
  warn!("Application started successfully");
  error!("An error occurred in the DB it might be shutting down");
  fatal!("An error occurred in the DB it might be shutting down");
  // error!("An error occurred in the DB it might be shutting down");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  // info!(user_id = user_id, username = username, "User logged in");

  // panic!("SIGINT received, shutting down!!");

  Ok(())
}

// use std::sync::Arc;
//
// use ttlog::{
//   file_listener::FileListener,
//   trace::Trace,
//   ttlog_macros::{error, info, warn},
// };
//
// pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
//   println!("TTLog Quick Start Example");
//
//   // Step 1: Initialize the tracing system
//   let trace = Trace::init(4096, 64, "default", Some("./tmp/"));
//   trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
//   trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
//
//   // Step 2: Use standard tracing macros to log
//   info!("Application started successfully");
//   warn!("Something might be wrong in the session handler");
//   error!("An error occurred in the DB it might be shutting down");
//
//   // Step 3: Log with structured data
//   let user_id = 42;
//   let username = "alice";
//   info!(user_id = user_id, username = username, "User logged in");
//
//   panic!("SIGINT received, shutting down!!");
//
//   println!("Done! Check ./tmp/ for ttlog-*.bin files");
//   println!("Run: ls -la ./tmp/ttlog-*.bin");
//
//   Ok(())
// }
