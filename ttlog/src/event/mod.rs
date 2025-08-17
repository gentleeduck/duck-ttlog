mod __test__;

use serde::de::{self, Visitor};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::fmt;

/// Represents the severity level of a log message.
/// Compatible with standard logging conventions.
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum LogLevel {
  /// Trace-level logging for very detailed debugging
  Trace = 0,
  /// Debug-level logging
  Debug = 1,
  /// Informational logging
  Info = 2,
  /// Warning-level logging
  Warn = 3,
  /// Error-level logging
  Error = 4,
}

/// A single log entry or event, storing all metadata efficiently.
#[derive(Debug, Clone)]
pub struct LogEvent {
  /// Timestamp of the event in nanoseconds
  pub timestamp_nanos: u64,
  /// Log severity level
  pub level: LogLevel,
  /// Logging target, e.g., module or subsystem
  pub target: &'static str,
  /// Main log message
  pub message: String,
  /// Structured fields attached to the event
  pub fields: SmallVec<[Field; 8]>,
  /// ID of the thread that emitted the event
  pub thread_id: u32,
  /// Source file where the log originated
  pub file: Option<&'static str>,
  /// Line number in the source file
  pub line: Option<u32>,
}

/// Represents a structured key/value field attached to a log event.
#[derive(Debug, Clone)]
pub struct Field {
  /// Field name
  pub key: &'static str,
  /// Field value
  pub value: FieldValue,
}

/// The value of a field, optimized for common types to avoid heap allocations.
/// Supports static strings, owned strings, numeric types, booleans, and debug/display text.
#[derive(Debug, Clone)]
pub enum FieldValue {
  /// Static string
  Str(&'static str),
  /// Owned string
  String(String),
  /// Signed 64-bit integer
  I64(i64),
  /// Unsigned 64-bit integer
  U64(u64),
  /// 64-bit floating point number
  F64(f64),
  /// Boolean value
  Bool(bool),
  /// Arbitrary debug output stored as a string
  Debug(String),
  /// Arbitrary display output stored as a string
  Display(String),
}

/// Builder pattern for constructing `LogEvent` instances efficiently.
pub struct EventBuilder {
  timestamp_nanos: u64,
  level: LogLevel,
  target: &'static str,
  message: String,
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
      target: "",
      message: String::new(),
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
  pub fn target(&mut self, target: &'static str) -> &mut Self {
    self.target = target;
    self
  }

  /// Sets the main log message.
  #[inline]
  pub fn message(&mut self, message: String) -> &mut Self {
    self.message = message;
    self
  }

  /// Adds a key/value field to the log event.
  #[inline]
  pub fn field(&mut self, key: &'static str, value: FieldValue) -> &mut Self {
    self.fields.push(Field { key, value });
    self
  }

  /// Builds the `LogEvent` consuming the builder.
  #[inline]
  pub fn build(self) -> LogEvent {
    LogEvent {
      timestamp_nanos: self.timestamp_nanos,
      level: self.level,
      target: self.target,
      message: self.message,
      fields: self.fields,
      thread_id: 0,
      file: None,
      line: None,
    }
  }
}

impl Serialize for FieldValue {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    match self {
      FieldValue::Str(s) => serializer.serialize_str(s),
      FieldValue::String(s) => serializer.serialize_str(s),
      FieldValue::I64(i) => serializer.serialize_i64(*i),
      FieldValue::U64(u) => serializer.serialize_u64(*u),
      FieldValue::F64(f) => serializer.serialize_f64(*f),
      FieldValue::Bool(b) => serializer.serialize_bool(*b),
      FieldValue::Debug(s) => serializer.serialize_str(s),
      FieldValue::Display(s) => serializer.serialize_str(s),
    }
  }
}

/// Visitor for deserializing `FieldValue`.
struct FieldValueVisitor;

impl<'de> Visitor<'de> for FieldValueVisitor {
  type Value = FieldValue;

  fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
    formatter.write_str("a valid FieldValue type")
  }

  fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(FieldValue::String(v.to_string()))
  }

  fn visit_i64<E>(self, v: i64) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(FieldValue::I64(v))
  }

  fn visit_u64<E>(self, v: u64) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(FieldValue::U64(v))
  }

  fn visit_f64<E>(self, v: f64) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(FieldValue::F64(v))
  }

  fn visit_bool<E>(self, v: bool) -> Result<Self::Value, E>
  where
    E: de::Error,
  {
    Ok(FieldValue::Bool(v))
  }
}

impl<'de> Deserialize<'de> for FieldValue {
  fn deserialize<D>(deserializer: D) -> Result<FieldValue, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    deserializer.deserialize_any(FieldValueVisitor)
  }
}
