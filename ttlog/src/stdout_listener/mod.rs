use std::io::{self, Write};
use std::sync::Arc;

use crate::event::LogEvent;
use crate::listener::LogListener;
use crate::string_interner::StringInterner;
use crate::trace::Trace;

/// Fast stdout listener with minimal allocations
pub struct StdoutListener {
  // Pre-allocate buffer to avoid allocations in hot path
  buffer: std::sync::Mutex<String>,
}

impl StdoutListener {
  pub fn new() -> Self {
    Self {
      buffer: std::sync::Mutex::new(String::with_capacity(256)),
    }
  }
}

impl LogListener for StdoutListener {
  fn handle(&self, event: &LogEvent, interner: &StringInterner) {
    if let Ok(mut buf) = self.buffer.try_lock() {
      buf.clear();

      // Resolve target
      let target_opt = interner.get_target(event.target_id);
      let target = match target_opt.as_deref() {
        Some(v) => v,
        None => "unknown",
      };

      // FIX: Handle None message_id properly
      let message: String = match event.message_id {
        Some(id) => match interner.get_message(id.get()) {
          Some(arc) => arc.to_string(),
          None => "<unknown_message>".to_string(),
        },
        None => {
          // For events without message, check if there's KV data
          match event.kv_id {
            Some(kv_id) => match interner.get_kv(kv_id.get()) {
              Some(kv_data) => match std::str::from_utf8(kv_data.as_slice()) {
                Ok(s) => s.to_string(),
                Err(_) => "<binary_kv_data>".to_string(),
              },
              None => "<structured_data>".to_string(),
            },
            None => "<no_message>".to_string(),
          }
        },
      };

      // Format log line
      use std::fmt::Write;
      let _ = write!(
        buf,
        "[{}] {}: {}\n",
        event.level().as_str(),
        target,
        message
      );

      // Single write call
      let _ = io::stdout().write_all(buf.as_bytes());
    }
  }
}

/// Initialize ttlog with stdout output - fastest setup
pub fn init_stdout() -> Result<(), Box<dyn std::error::Error>> {
  let trace = Trace::init(4096, 64, "default", Some("./logs/"));
  trace.add_listener(Arc::new(StdoutListener::new()));
  Ok(())
}

/// Initialize with custom capacity
pub fn init_stdout_with_capacity(capacity: usize) -> Result<(), Box<dyn std::error::Error>> {
  let trace = Trace::init(capacity, 64, "default", Some("./logs/"));
  trace.add_listener(Arc::new(StdoutListener::new()));
  Ok(())
}
