mod __test__;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;

/// Log levels - compatible with standard logging
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
  Trace = 0,
  Debug = 1,
  Info = 2,
  Warn = 3,
  Error = 4,
}

/// Efficient event representation
#[derive(Debug, Clone)]
pub struct LogEvent {
  pub timestamp_nanos: u64,
  pub level: LogLevel,
  pub target: &'static str,
  pub message: String,
  pub fields: SmallVec<[Field; 8]>, // Most events have <8 fields
  pub thread_id: u32,
  pub file: Option<&'static str>,
  pub line: Option<u32>,
}

/// Optimized field storage
#[derive(Debug, Clone)]
pub struct Field {
  pub key: &'static str,
  pub value: FieldValue,
}

/// Efficient value storage - avoid Box/heap allocation for common types
#[derive(Debug, Clone)]
pub enum FieldValue {
  Str(&'static str),
  String(String),
  I64(i64),
  U64(u64),
  F64(f64),
  Bool(bool),
  Debug(String),   // For types that only implement Debug
  Display(String), // For types that implement Display
}

pub struct EventBuilder {
  timestamp_nanos: u64,
  level: LogLevel,
  target: &'static str,
  message: String,
  fields: SmallVec<[Field; 8]>,
}

impl EventBuilder {
  #[inline]
  pub fn new_with_capacity(field_count: usize) -> Self {
    Self {
      timestamp_nanos: 0,
      level: LogLevel::Info,
      target: "",
      message: String::new(),
      fields: SmallVec::with_capacity(field_count),
    }
  }

  #[inline]
  pub fn timestamp_nanos(&mut self, timestamp_nanos: u64) -> &mut Self {
    self.timestamp_nanos = timestamp_nanos;
    self
  }

  #[inline]
  pub fn level(&mut self, level: LogLevel) -> &mut Self {
    self.level = level;
    self
  }

  #[inline]
  pub fn target(&mut self, target: &'static str) -> &mut Self {
    self.target = target;
    self
  }

  #[inline]
  pub fn message(&mut self, message: String) -> &mut Self {
    self.message = message;
    self
  }

  #[inline]
  pub fn field(&mut self, key: &'static str, value: FieldValue) -> &mut Self {
    self.fields.push(Field { key, value });
    self
  }

  #[inline]
  pub fn build(self) -> LogEvent {
    LogEvent {
      timestamp_nanos: self.timestamp_nanos,
      level: self.level,
      target: self.target,
      message: self.message,
      fields: self.fields,
      thread_id: 0, // Not yet supported
      file: None,   // Not yet supported NOTE: will make macro for this
      line: None,
    }
  }
}
