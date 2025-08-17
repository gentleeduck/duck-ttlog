mod __test__;

use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::{borrow::Cow, fmt};

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
  Trace = 0,
  Debug = 1,
  Info = 2,
  Warn = 3,
  Error = 4,
}

impl LogLevel {
  pub fn get_typo(level: &str) -> LogLevel {
    match level {
      "trace" => LogLevel::Trace,
      "debug" => LogLevel::Debug,
      "info" => LogLevel::Info,
      "warn" => LogLevel::Warn,
      "error" => LogLevel::Error,
      // Why? - because there should be typo level
      _ => LogLevel::Info,
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  pub timestamp_nanos: u64,
  pub level: LogLevel,
  pub target: Cow<'static, str>,
  pub message: Cow<'static, str>,
  #[serde(bound(
    serialize = "SmallVec<[Field; 8]>: Serialize",
    deserialize = "SmallVec<[Field; 8]>: Deserialize<'de>"
  ))]
  pub fields: SmallVec<[Field; 8]>,
  pub thread_id: u32,
  pub file: Option<Box<str>>,
  pub line: Option<u32>,
}

impl<'a> fmt::Display for LogEvent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(f, "{}", self.message)
  }
}

/// Represents a structured key/value field attached to a log event.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Field {
  pub key: Cow<'static, str>,
  pub value: FieldValue,
}

#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "type", content = "value")]
pub enum FieldValue {
  Str(Cow<'static, str>),
  String(String),
  Debug(String),
  Display(String),
  Null,
  None,
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
}

/// Builder pattern for constructing `LogEvent` instances efficiently.
#[derive(Debug, Clone)]
pub struct EventBuilder {
  timestamp_nanos: u64,
  level: LogLevel,
  target: Cow<'static, str>,
  message: Cow<'static, str>,
  fields: SmallVec<[Field; 8]>,
}

impl EventBuilder {
  /// Creates a new event builder with pre-allocated capacity for fields.
  ///
  /// # Arguments
  /// * `field_count` - Estimated number of fields for allocation optimization
  ///
  /// # Example
  /// ```
  /// use ttlog::event::EventBuilder;
  /// let mut builder = EventBuilder::new_with_capacity(4);
  /// builder.message("Hello".into());
  /// ```
  #[inline]
  pub fn new_with_capacity(field_count: usize) -> Self {
    Self {
      timestamp_nanos: 0,
      level: LogLevel::Info,
      target: "".into(),
      message: "".into(),
      fields: SmallVec::with_capacity(field_count),
    }
  }

  /// Sets the timestamp in nanoseconds.
  #[inline]
  pub fn timestamp_nanos(&mut self, timestamp_nanos: u64) -> &mut Self {
    self.timestamp_nanos = timestamp_nanos;
    self
  }

  /// Sets the log level.
  #[inline]
  pub fn level(&mut self, level: LogLevel) -> &mut Self {
    self.level = level;
    self
  }

  /// Sets the logging target.
  #[inline]
  pub fn target(&mut self, target: Cow<'static, str>) -> &mut Self {
    self.target = target;
    self
  }

  /// Sets the main log message.
  #[inline]
  pub fn message(&mut self, message: Cow<'static, str>) -> &mut Self {
    self.message = message;
    self
  }

  /// Adds a key/value field to the log event.
  #[inline]
  pub fn field(&mut self, key: &'static str, value: FieldValue) -> &mut Self {
    self.fields.push(Field {
      key: Cow::Borrowed(key),
      value,
    });
    self
  }

  /// Builds the `LogEvent` consuming the builder.
  #[inline]
  pub fn build(&mut self) -> LogEvent {
    LogEvent {
      timestamp_nanos: self.timestamp_nanos,
      level: self.level,
      target: std::mem::take(&mut self.target),
      message: std::mem::take(&mut self.message),
      fields: self.fields.clone(),
      thread_id: 0,
      file: None,
      line: None,
    }
  }

  pub fn reset(&mut self) -> &mut Self {
    self.timestamp_nanos = 0;
    self.level = LogLevel::Info;
    self.target = Cow::Borrowed("");
    self.message = Cow::Borrowed("");
    self.fields.clear();
    self
  }
}
