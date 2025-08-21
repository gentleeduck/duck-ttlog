use std::fs::OpenOptions;
use std::io::{self, Write};
use std::path::Path;
use std::sync::{Arc, Mutex};

use crate::event::LogEvent;
use crate::listener::LogListener;
use crate::string_interner::StringInterner;
use crate::trace::Trace;

/// File listener for structured logs
pub struct FileListener {
  buffer: Mutex<String>,
  file: Mutex<std::fs::File>,
}

impl FileListener {
  /// Open (or create if missing) file for appending logs
  pub fn new(path: &str) -> io::Result<Self> {
    let path_obj = Path::new(path);

    // Ensure parent directories exist
    if let Some(parent) = path_obj.parent() {
      std::fs::create_dir_all(parent)?;
    }

    // Open existing file, or create if missing
    let file = OpenOptions::new()
      .create(true) // only creates if it doesn’t exist
      .write(true) // allow writing
      .append(true) // don’t truncate, just append
      .open(path_obj)?;

    Ok(Self {
      buffer: Mutex::new(String::with_capacity(256)),
      file: Mutex::new(file),
    })
  }
}

impl LogListener for FileListener {
  fn handle(&self, event: &LogEvent, interner: &StringInterner) {
    if let Ok(mut buf) = self.buffer.lock() {
      buf.clear();

      // Resolve strings
      let target_opt = interner.get_target(event.target_id);
      let target = target_opt.as_deref().unwrap_or("unknown");

      let message_opt = interner.get_message(match event.message_id {
        Some(v) => v,
        None => {
          eprintln!("[Trace] Unknown message id: {}", event.message_id.unwrap());
          return;
        },
      });
      let message = message_opt.as_deref().unwrap_or("unknown");

      // Format log line
      use std::fmt::Write;
      let _ = write!(
        buf,
        "[{}] {}: {}\n",
        event.level().as_str(),
        target,
        message
      );

      // Write to file (one syscall)
      if let Ok(mut file) = self.file.lock() {
        let _ = file.write_all(buf.as_bytes());
      }
    }
  }
}

/// Initialize ttlog with file output
pub fn init_file(path: &str) -> Result<(), Box<dyn std::error::Error>> {
  let trace = Trace::init(4096, 64, "default", Some("./logs/"));
  trace.add_listener(Arc::new(FileListener::new(path)?));
  Ok(())
}
