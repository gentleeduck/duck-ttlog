use std::sync::Arc;
use ttlog::{
  file_listener::FileListener,
  stdout_listener::StdoutListener,
  trace::Trace,
  ttlog_macros::{debug, error, fatal, info, trace, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut trace = Trace::init(2, 64, "example_simple", Some("./tmp"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(StdoutListener::new()));
  trace.set_level(ttlog::event::LogLevel::TRACE);

  let handle = std::thread::spawn(|| loop {
    debug!("Waiting for compaction");
    std::thread::sleep(std::time::Duration::from_secs(1));
  });
  handle.join().unwrap();

  // trace!("Application started successfullyy");
  // debug!("Application started successfullyy");
  // info!("Application started successfullyyy");
  // warn!("Application started successfullyyyy");
  // error!("An error occurred in the DB it might be shutting down");
  // fatal!("An error occurred in the DB it might be shutting down");

  let user_id = 42;
  let username = "alice";
  let x = &username;
  info!(user_id = user_id, username = *x, "User logged in");

  // panic!("SIGINT received, shutting down!!");
  std::thread::sleep(std::time::Duration::from_secs(10));
  trace.shutdown();

  Ok(())
}
