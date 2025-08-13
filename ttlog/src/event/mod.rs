mod __test__;
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Event {
  pub ts: u64,
  pub level: u8,
  pub message: String,
}

impl Event {
  pub fn new(ts: u64, level: u8, message: String) -> Self {
    Self { ts, level, message }
  }

  pub fn serialize(&self) -> String {
    serde_json::to_string(self).expect("Failed to serialize")
  }

  pub fn deserialize(json: String) -> Self {
    serde_json::from_str::<Self>(&json).expect("Failed to deserialize")
  }
}
