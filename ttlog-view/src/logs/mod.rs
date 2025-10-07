use serde::{Deserialize, Serialize};
use std::{
  fs,
  io::{self, BufRead},
};
use ttlog::event::LogLevel;

use crate::utils::Utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileEvent {
  pub file: String,
  pub kv: serde_json::Value,
  pub level: u8,
  pub message: String,
  pub position: (u32, u32),
  pub target: String,
  pub thread_id: u8,
  pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedLog {
  pub level: LogLevel,
  pub timestamp: String,
  pub thread_id: u8,
  pub message: String,
  pub target: String,
  pub kv: serde_json::Value,
  pub file: String,
  pub position: (u32, u32),
}

pub struct Logs;

impl Logs {
  pub fn get_logs(path: &str) -> Vec<ResolvedLog> {
    let file = fs::File::open(path).unwrap();
    let reader = io::BufReader::new(file);
    let mut events: Vec<ResolvedLog> = Vec::new();

    for line in reader.lines() {
      let line = line.unwrap();

      // Deserialize the line directly as a LogFileEvent (new format)
      let log_event: LogFileEvent = serde_json::from_str(&line).unwrap();

      // Convert to our internal ResolvedEvent format
      let level = LogLevel::from_u8(&log_event.level);
      let timestamp = Utils::format_timestamp(log_event.timestamp);

      let event = ResolvedLog {
        level,
        timestamp,
        thread_id: log_event.thread_id,
        message: log_event.message,
        target: log_event.target,
        kv: log_event.kv,
        file: log_event.file,
        position: log_event.position,
      };

      events.push(event);
    }

    events
  }
}
