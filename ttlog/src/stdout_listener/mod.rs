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
    // Fast format without allocating new strings
    if let Ok(mut buf) = self.buffer.try_lock() {
      buf.clear();

      // Resolve strings once
      let target_opt = interner.get_target(event.target_id);
      let target = target_opt.as_deref().unwrap_or("unknown");

      let message_opt = interner.get_message(match event.message_id {
        Some(v) => v.get(),
        None => {
          eprintln!("[Trace] Unknown message id: {}", event.message_id.unwrap());
          return;
        },
      });
      let message = message_opt.as_deref().unwrap_or("unknown");

      // Fast format - no heap allocation
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
    // If lock contention, just drop the event (performance over guarantees)
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
