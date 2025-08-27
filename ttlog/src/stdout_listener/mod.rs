use chrono::{DateTime, TimeZone, Utc};
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

      // Resolve Target
      let target: String = interner
        .get_target(event.target_id)
        .map(|t| t.to_string())
        .unwrap_or_else(|| "unknown".to_string());

      // Resolve Message or KV Data
      let message: String = match event.message_id {
        Some(id) => interner
          .get_message(id.get())
          .map(|arc| arc.to_string())
          .unwrap_or_else(|| "<unknown_message>".to_string()),
        None => match event.kv_id {
          Some(kv_id) => match interner.get_kv(kv_id.get()) {
            Some(kv_data) => match std::str::from_utf8(kv_data.as_slice()) {
              Ok(s) => s.to_string(),
              Err(_) => "<binary_kv_data>".to_string(),
            },
            None => "<structured_data>".to_string(),
          },
          None => "<no_message>".to_string(),
        },
      };

      // Resolve File + Position
      let file: String = interner
        .get_file(event.file_id)
        .map(|f| f.to_string())
        .unwrap_or_else(|| "unknown".to_string());

      let (line, col) = event.position;

      // Decode packed_meta (timestamp, level, thread_id) ─
      let ts_ms = event.timestamps();
      let level = event.level(); // from packed_meta
      let thread_id = event.thread_id(); // from packed_meta

      // Convert timestamp

      let datetime: DateTime<Utc> =
        DateTime::from_timestamp((ts_ms / 1000) as i64, ((ts_ms % 1000) * 1_000_000) as u32)
          .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());
      let formatted_ts = datetime.format("%Y-%m-%d %H:%M:%S%.3f");

      // Format full log line
      use std::fmt::Write;
      let _ = write!(
        buf,
        "[{time}][{level:^5}][thread:{tid}] {target} @ {file}:{line}:{col} | {msg}\n",
        time = formatted_ts,
        level = level.as_str(),
        tid = thread_id,
        target = target,
        file = file,
        line = line,
        col = col,
        msg = message,
      );

      // Single stdout write
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
