// use std::sync::Arc;
// use uuid::Uuid;
//
// use ttlog::{
//   file_listener::FileListener,
//   trace::Trace,
//   ttlog_macros::{error, info, warn},
// };
//
// fn cart_service(trace: &Trace, user_id: i32) {
//   let trace_id = Uuid::new_v4(); // unique ID for this request
//
//   info!(
//     trace_id = trace_id,
//     user_id = user_id,
//     "Cart service: received checkout request"
//   );
//
//   // simulate calling another service (auth)
//   auth_service(trace_id, user_id);
// }
//
// fn auth_service(trace_id: Uuid, user_id: i32) {
//   // in real distributed system, `trace_id` comes via HTTP header / MQ metadata
//   let trace = Trace::init(4096, 64, "auth_service", Some("./tmp2/"));
//   trace.add_listener(Arc::new(FileListener::new("./tmp2/ttlog.log").unwrap()));
//
//   info!(
//     trace_id = trace_id,
//     user_id = user_id,
//     "Auth service: validating user"
//   );
//
//   if user_id == 42 {
//     warn!(
//       trace_id = trace_id,
//       user_id = user_id,
//       "User flagged for suspicious activity"
//     );
//   } else {
//     error!(
//       trace_id = trace_id,
//       user_id = user_id,
//       "Unknown user login attempt"
//     );
//   }
// }
//
// pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
//   println!("TTLog Distributed System Example");
//
//   // Service A: cart_service
//   let trace = Trace::init(4096, 64, "cart_service", Some("./tmp/"));
//   trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
//   trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
//
//   // Simulate a user action in cart service
//   cart_service(&trace, 42);
//
//   println!("Logs written with trace_id. Check ./tmp/ and ./tmp2/");
//   Ok(())
// }

use std::sync::Arc;

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{error, info, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  println!("TTLog Quick Start Example");

  // Step 1: Initialize the tracing system
  let trace = Trace::init(4096, 64, "default", Some("./tmp/"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));

  // Step 2: Use standard tracing macros to log
  info!("Application started successfully");
  warn!("Something might be wrong in the session handler");
  error!("An error occurred in the DB it might be shutting down");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  info!(user_id = user_id, username = username, "User logged in");

  panic!("SIGINT received, shutting down!!");

  println!("Done! Check ./tmp/ for ttlog-*.bin files");
  println!("Run: ls -la ./tmp/ttlog-*.bin");

  Ok(())
}
