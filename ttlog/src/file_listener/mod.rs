use crate::event::LogEvent;
use crate::listener::LogListener;
use crate::string_interner::StringInterner;
use std::fs::OpenOptions;
use std::io::{self, Write as IoWrite};
use std::path::Path;
use std::sync::Mutex;

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

    let file = OpenOptions::new()
      .create(true)
      .write(true)
      .append(true)
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

      // Keep Arc values alive by binding them to variables
      let target_arc = interner.get_target(event.target_id);
      let target = target_arc.as_deref().unwrap_or("unknown");

      let message_arc = event.message_id.and_then(|v| interner.get_message(v.get()));
      let message = message_arc.as_deref().unwrap_or("unknown");

      let file_arc = interner.get_file(event.file_id);
      let file = file_arc.as_deref().unwrap_or("unknown");

      let kv_data = event.kv_id.and_then(|id| {
        interner.get_kv(id.get()).and_then(|bytes| {
          std::str::from_utf8(&bytes)
            .ok()
            .and_then(|s| serde_json::from_str::<serde_json::Value>(s).ok())
        })
      });

      let (timestamp, level, thread_id) = LogEvent::unpack_meta(event.packed_meta);

      // Build structured JSON object
      let log_json = serde_json::json!({
          "timestamp": timestamp,
          "level": level,
          "thread_id": thread_id,
          "file": file,
          "kv": kv_data,
          "target": target,
          "message": message,
          "position": event.position,
      });

      // Serialize as compact JSON (not pretty) to make one JSON object per line
      match serde_json::to_string(&log_json) {
        Ok(line) => {
          buf.push_str(&line);
          buf.push('\n'); // newline-delimited JSON (NDJSON)
          if let Ok(mut file) = self.file.lock() {
            let _ = file.write_all(buf.as_bytes());
          }
        },
        Err(err) => {
          eprintln!("[Trace] Failed to serialize log JSON: {}", err);
        },
      }
    }
  }
}
