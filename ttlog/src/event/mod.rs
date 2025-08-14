mod __test__;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum Level {
  Trace,
  Debug,
  Info,
  Warn,
  Error,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
  pub timestamp: u64,
  pub level: String,
  pub message: String,
  pub target: String,
}

impl Event {
  pub fn new(timestamp: u64, level: String, message: String, target: String) -> Self {
    Self {
      timestamp,
      level,
      message,
      target,
    }
  }

  pub fn serialize(&self) -> String {
    serde_json::to_string(self).expect("Failed to serialize")
  }

  pub fn deserialize(json: String) -> Self {
    serde_json::from_str::<Self>(&json).expect("Failed to deserialize")
  }
}

impl Default for Event {
  fn default() -> Self {
    Self {
      timestamp: 0,
      level: "".to_string(), // Level::Info,
      message: String::new(),
      target: String::new(),
    }
  }
}
