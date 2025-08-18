mod __test__;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{borrow::Cow, fmt};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
  TRACE = 0,
  DEBUG = 1,
  INFO = 2,
  WARN = 3,
  ERROR = 4,
}

impl LogLevel {
  #[inline]
  pub fn from_str(level: &str) -> LogLevel {
    match level {
      "trace" => LogLevel::TRACE,
      "debug" => LogLevel::DEBUG,
      "info" => LogLevel::INFO,
      "warn" => LogLevel::WARN,
      "error" => LogLevel::ERROR,
      _ => LogLevel::INFO,
    }
  }
}

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
#[serde(tag = "type", content = "value")]
pub enum FieldValue {
  Bool(bool),
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  F32(f32),
  F64(f64),
  StringId(u16),
}

/// Compact field - only 12 bytes total
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct Field {
  pub key_id: u16,       // 2 bytes - interned key
  pub value: FieldValue, // 10 bytes max
}

impl Field {
  #[inline]
  const fn empty() -> Self {
    Self {
      key_id: 0,
      value: FieldValue::Bool(false),
    }
  }
}

impl fmt::Display for LogEvent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Event(target_id={}, message_id={})",
      self.target_id, self.message_id
    )
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  // Core metadata packed into 64 bits
  pub packed_meta: u64, // 8 bytes: [timestamp:52][level:4][thread_id:8]

  // Interned string references
  pub target_id: u16,  // 2 bytes
  pub message_id: u16, // 2 bytes

  // Compact field storage (most events have 0-2 fields)
  pub field_count: u8,    // 1 byte
  pub fields: [Field; 3], // 36 bytes (3 * 12 bytes)

  // Optional debug info (only when needed)
  pub file_id: u16, // 2 bytes - interned filename
  pub line: u16,    // 2 bytes

  // Padding to reach 64 bytes (cache line aligned)
  _padding: [u8; 9], // 9 bytes padding
}

impl LogEvent {
  #[inline]
  pub fn timestamp_millis(&self) -> u64 {
    self.packed_meta >> 12
  }

  #[inline]
  pub fn level(&self) -> LogLevel {
    unsafe { std::mem::transmute(((self.packed_meta >> 8) & 0xF) as u8) }
  }

  #[inline]
  pub fn thread_id(&self) -> u8 {
    (self.packed_meta & 0xFF) as u8
  }

  #[inline]
  pub fn pack_meta(timestamp_millis: u64, level: LogLevel, thread_id: u8) -> u64 {
    // Ensure timestamp fits in 52 bits
    (timestamp_millis << 12) | ((level as u64) << 8) | (thread_id as u64)
  }

  #[inline]
  pub fn target(&mut self, target_id: u16) -> &mut Self {
    self.target_id = target_id;
    self
  }

  /// Create empty event for building
  #[inline]
  pub fn new() -> Self {
    Self {
      packed_meta: 0,
      target_id: 0,
      message_id: 0,
      field_count: 0,
      fields: [Field::empty(); 3],
      file_id: 0,
      line: 0,
      _padding: [0; 9],
    }
  }

  /// Add a field if there's space (up to 3 fields)
  #[inline]
  pub fn add_field(&mut self, key_id: u16, value: FieldValue) -> bool {
    if self.field_count < 3 {
      self.fields[self.field_count as usize] = Field { key_id, value };
      self.field_count += 1;
      true
    } else {
      false // Silently drop excess fields
    }
  }

  /// Reset for reuse (object pooling)
  #[inline]
  pub fn reset(&mut self) {
    self.packed_meta = 0;
    self.target_id = 0;
    self.message_id = 0;
    self.field_count = 0;
    self.file_id = 0;
    self.line = 0;
    // Don't need to clear fields array - field_count handles validity
  }

  #[inline]
  pub fn unpack_meta(meta: u64) -> (u64, u8, u8) {
    let timestamp = meta >> 12;
    let level = ((meta >> 8) & 0xF) as u8;
    let thread_id = (meta & 0xFF) as u8;
    (timestamp, level, thread_id)
  }
}
