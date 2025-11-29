use chrono::{DateTime, TimeZone, Utc};
use std::io::{self, Write};

use crate::event::LogEvent;
use crate::listener::LogListener;
use crate::string_interner::StringInterner;

const RESET: &str = "\x1b[0m";
const RED: &str = "\x1b[31m";
const GREEN: &str = "\x1b[32m";
const YELLOW: &str = "\x1b[33m";
const BLUE: &str = "\x1b[34m";
const MAGENTA: &str = "\x1b[35m";
const CYAN: &str = "\x1b[36m";
const WHITE: &str = "\x1b[37m";

pub struct StdoutListener {
  buffer: std::sync::Mutex<String>,
}

impl StdoutListener {
  pub fn new() -> Self {
    Self {
      buffer: std::sync::Mutex::new(String::with_capacity(256)),
    }
  }
}

impl Default for StdoutListener {
  fn default() -> Self {
    Self::new()
  }
}

impl LogListener for StdoutListener {
  fn handle(&self, event: &LogEvent, interner: &StringInterner) {
    if let Ok(mut buf) = self.buffer.try_lock() {
      buf.clear();

      let target: String = {
        let this = interner.get_target(event.target_id).map(|t| t.to_string());
        match this {
          Some(x) => x,
          None => "".to_string(),
        }
      };

      let message: String = match event.message_id {
        Some(id) => {
          let this = interner.get_message(id.get()).map(|arc| arc.to_string());
          match this {
            Some(x) => x,
            None => "".to_string(),
          }
        },
        None => "".to_string(),
      };

      let kv: String = match event.kv_id {
        Some(kv_id) => match interner.get_kv(kv_id.get()) {
          Some(kv_data) => match std::str::from_utf8(kv_data.as_slice()) {
            Ok(s) => s.to_string(),
            Err(_) => "".to_string(),
          },
          None => "".to_string(),
        },
        None => "".to_string(),
      };

      let (line, col) = event.position;

      let ts_ms = event.timestamps();
      let level = event.level();
      let thread_id = event.thread_id();

      let datetime: DateTime<Utc> =
        DateTime::from_timestamp((ts_ms / 1000) as i64, ((ts_ms % 1000) * 1_000_000) as u32)
          .unwrap_or_else(|| Utc.timestamp_opt(0, 0).unwrap());

      let level_colored = color_level(level.as_str());
      let target_colored = format!("{}{}{}", MAGENTA, target, RESET);
      let msg_colored = format!("{}{}{}", WHITE, message, RESET);
      let kv_colored = format!("{}{}{}", BLUE, kv, RESET);

      use std::fmt::Write;
      let _ = writeln!(
        buf,
        "{time_color}[{time}]{reset} {level} {thread_color}t{tid}{reset} {target}:{line}:{col} {msg} {kv}",
        time_color = GREEN,
        reset = RESET,
        level = level_colored,
        thread_color = CYAN,
        tid = thread_id,
        time = datetime.format("%H:%M:%S%.3f"),
        target = target_colored,
        line = line,
        col = col,
        msg = msg_colored,
        kv = kv_colored
      );

      let _ = io::stdout().write_all(buf.as_bytes());
    }
  }
}

fn color_level(level: &str) -> String {
  match level {
    "ERROR" => format!("{}[{}]{}", RED, level, RESET),
    "WARN" => format!("{}[{}]{}", YELLOW, level, RESET),
    "INFO" => format!("{}[{}]{}", GREEN, level, RESET),
    "DEBUG" => format!("{}[{}]{}", BLUE, level, RESET),
    "TRACE" => format!("{}[{}]{}", CYAN, level, RESET),
    "FATAL" => format!("{}[{}]{}", RED, level, RESET),
    _ => level.to_string(),
  }
}
