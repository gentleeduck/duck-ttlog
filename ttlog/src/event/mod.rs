mod __test__;

use core::num;
use serde::{Deserialize, Serialize};
use std::fmt;

#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
  TRACE = 0,
  DEBUG = 1,
  INFO = 2,
  WARN = 3,
  ERROR = 4,
  FATAL = 5,
}

impl LogLevel {
  #[inline]
  pub fn from_u8(level: &u8) -> LogLevel {
    match level {
      0 => LogLevel::TRACE,
      1 => LogLevel::DEBUG,
      2 => LogLevel::INFO,
      3 => LogLevel::WARN,
      4 => LogLevel::ERROR,
      5 => LogLevel::FATAL,
      _ => LogLevel::INFO,
    }
  }

  pub fn from_u8_to_str(level: &u8) -> &'static str {
    match level {
      0 => "TRACE",
      1 => "DEBUG",
      2 => "INFO",
      3 => "WARN",
      4 => "ERROR",
      5 => "FATAL",
      _ => "INFO",
    }
  }

  pub fn as_str(&self) -> &'static str {
    match self {
      Self::TRACE => "TRACE",
      Self::DEBUG => "DEBUG",
      Self::INFO => "INFO",
      Self::WARN => "WARN",
      Self::ERROR => "ERROR",
      Self::FATAL => "FATAL",
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

#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct Field {
  pub key_id: u16,
  pub value: FieldValue,
}

impl Field {
  #[inline]
  pub const fn empty() -> Self {
    Self {
      key_id: 0,
      value: FieldValue::Bool(false),
    }
  }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  pub packed_meta: u64,
  pub target_id: u16,
  pub message_id: Option<num::NonZeroU16>,
  pub kv_id: Option<num::NonZeroU16>,
  pub file_id: u16,
  pub position: (u32, u32),
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
    (timestamp_millis << 12) | ((level as u64) << 8) | (thread_id as u64)
  }

  #[inline]
  pub fn target(&mut self, target_id: u16) -> &mut Self {
    self.target_id = target_id;
    self
  }

  #[inline]
  pub fn new() -> Self {
    Self {
      packed_meta: 0,
      target_id: 0,
      message_id: num::NonZeroU16::new(0),
      kv_id: num::NonZeroU16::new(0),
      file_id: 0,
      position: (0, 0),
    }
  }

  #[inline]
  pub fn reset(&mut self) {
    self.packed_meta = 0;
    self.target_id = 0;
    self.message_id = num::NonZeroU16::new(0);
    self.kv_id = num::NonZeroU16::new(0);
    self.file_id = 0;
    self.position = (0, 0);
    // Note: fields array is not cleared for performance -
    // it will be overwritten as field_count increases
  }

  #[inline]
  pub fn unpack_meta(meta: u64) -> (u64, u8, u8) {
    let timestamp = meta >> 12;
    let level = ((meta >> 8) & 0xF) as u8;
    let thread_id = (meta & 0xFF) as u8;
    (timestamp, level, thread_id)
  }
}

const _: () = {
  assert!(std::mem::size_of::<LogEvent>() == 24);
  assert!(std::mem::align_of::<LogEvent>() >= 8);
};
