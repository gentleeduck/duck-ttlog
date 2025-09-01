use std::{ops::Deref, sync::Arc};

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{debug, error, fatal, info, trace, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  println!("TTLog Quick Start Example");

  let trace = Trace::init(4096, 64, "test", Some("./tmp"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));

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
  info!(user_id = user_id, username = username, "User logged in");

  panic!("SIGINT received, shutting down!!");

  Ok(())
}

fn foo() {
  let y = DuckBox::new(String::from("hello"));
  with_string(&y);

  println!("{:?} {:?}", &x, z);
}

fn with_string(x: &str) -> &str {
  x
}

#[derive(Debug)]
struct DuckBox<T>(T);

impl<T> DuckBox<T> {
  pub fn new(v: T) -> Self {
    Self(v)
  }
}

impl<T> Deref for DuckBox<T> {
  type Target = T;
  fn deref(&self) -> &Self::Target {
    &self.0
  }
}
