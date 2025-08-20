mod __test__;

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
  pub message_id: u16,
  pub field_count: u8,
  pub fields: [Field; 3],
  pub file_id: u16,
  pub line: u16,
  pub _padding: [u8; 9],
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
      message_id: 0,
      field_count: 0,
      fields: [Field::empty(); 3],
      file_id: 0,
      line: 0,
      _padding: [0; 9],
    }
  }

  #[inline]
  pub fn add_field(&mut self, key_id: u16, value: FieldValue) -> bool {
    if self.field_count < 3 {
      self.fields[self.field_count as usize] = Field { key_id, value };
      self.field_count += 1;
      true
    } else {
      false
    }
  }

  #[inline]
  pub fn reset(&mut self) {
    self.packed_meta = 0;
    self.target_id = 0;
    self.message_id = 0;
    self.field_count = 0;
    self.file_id = 0;
    self.line = 0;
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

impl fmt::Display for LogEvent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Event(target_id={}, message_id={})",
      self.target_id, self.message_id
    )
  }
}

#[derive(Debug, Default)]
pub struct EventMetrics {
  pub events_created: std::sync::atomic::AtomicU64,
  pub total_build_time_ns: std::sync::atomic::AtomicU64,
  pub cache_hits: std::sync::atomic::AtomicU64,
  pub cache_misses: std::sync::atomic::AtomicU64,
}

impl EventMetrics {
  pub fn record_build_time(&self, start: std::time::Instant) {
    let elapsed_ns = start.elapsed().as_nanos() as u64;
    self
      .total_build_time_ns
      .fetch_add(elapsed_ns, std::sync::atomic::Ordering::Relaxed);
    self
      .events_created
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  pub fn avg_build_time_ns(&self) -> u64 {
    let total = self
      .total_build_time_ns
      .load(std::sync::atomic::Ordering::Relaxed);
    let count = self
      .events_created
      .load(std::sync::atomic::Ordering::Relaxed);

    if count > 0 {
      total / count
    } else {
      0
    }
  }

  #[inline]
  pub fn record_cache_hit(&self) {
    self
      .cache_hits
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  #[inline]
  pub fn record_cache_miss(&self) {
    self
      .cache_misses
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  pub fn cache_hit_rate(&self) -> f64 {
    let hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
    let misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
    let total = hits + misses;

    if total > 0 {
      (hits as f64 / total as f64) * 100.0
    } else {
      0.0
    }
  }
}

const _: () = {
  assert!(std::mem::size_of::<LogEvent>() == 104);
  assert!(std::mem::align_of::<LogEvent>() >= 8);
};
