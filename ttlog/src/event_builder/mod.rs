use std::num::NonZeroU16;
use std::sync::Arc;

use smallvec::SmallVec;

use crate::event::{FieldValue, LogEvent, LogLevel};
use crate::string_interner::StringInterner;

/// Helper for constructing `LogEvent` instances outside of the macro pipeline.
///
/// The logging macros normally own the logic for interning strings and packing
/// metadata. The builder keeps the tests (and any potential manual callers)
/// decoupled from those macros while still exercising the same code paths.
pub struct EventBuilder {
  interner: Arc<StringInterner>,
}

impl EventBuilder {
  pub fn new(interner: Arc<StringInterner>) -> Self {
    Self { interner }
  }

  #[inline]
  pub fn build_fast(
    &self,
    timestamp: u64,
    level: LogLevel,
    target: &str,
    message: &str,
  ) -> LogEvent {
    self.build_event(timestamp, level, target, message, None)
  }

  pub fn build_with_fields(
    &self,
    timestamp: u64,
    level: LogLevel,
    target: &str,
    message: &str,
    fields: &[(String, FieldValue)],
  ) -> LogEvent {
    if fields.is_empty() {
      return self.build_fast(timestamp, level, target, message);
    }

    let mut serialized = serde_json::Map::with_capacity(fields.len());
    for (key, value) in fields {
      serialized.insert(key.clone(), value.to_json_value());
    }
    let json = serde_json::Value::Object(serialized).to_string();
    let mut buf: SmallVec<[u8; 128]> = SmallVec::with_capacity(json.len());
    buf.extend_from_slice(json.as_bytes());

    let kv_id = self.interner.intern_kv(buf);
    let kv_id = NonZeroU16::new(kv_id);

    self.build_event(timestamp, level, target, message, kv_id)
  }

  fn build_event(
    &self,
    timestamp: u64,
    level: LogLevel,
    target: &str,
    message: &str,
    kv_id: Option<NonZeroU16>,
  ) -> LogEvent {
    let target_id = self.interner.intern_target(target);
    let message_id = self.interner.intern_message(message);

    LogEvent {
      packed_meta: LogEvent::pack_meta(timestamp, level, 0),
      target_id,
      message_id: NonZeroU16::new(message_id),
      kv_id,
      file_id: 0,
      position: (0, 0),
    }
  }
}
