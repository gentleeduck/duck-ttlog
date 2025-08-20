mod __test__;
use std::{sync::Arc, thread};

use crate::{
  event::{FieldValue, LogEvent, LogLevel},
  string_interner::StringInterner,
};

#[derive(Debug)]
pub struct EventBuilder {
  interner: Arc<StringInterner>,
  thread_id: u8,
  event_pool: Vec<LogEvent>,
  pool_index: usize,
}

impl EventBuilder {
  pub fn new(interner: Arc<StringInterner>) -> Self {
    let thread_id = EventBuilder::current_thread_id_u32() as u8;

    Self {
      interner,
      thread_id,
      event_pool: Vec::with_capacity(16),
      pool_index: 0,
    }
  }

  pub fn get_pooled_event(&mut self) -> &mut LogEvent {
    if self.event_pool.len() <= self.pool_index {
      self.event_pool.push(LogEvent::new());
    }

    let event = &mut self.event_pool[self.pool_index];
    self.pool_index = (self.pool_index + 1) % 16;

    event.reset();
    event
  }

  #[inline]
  pub fn build_fast(
    &mut self,
    timestamp_millis: u64,
    level: LogLevel,
    target: &str,
    message: &str,
  ) -> LogEvent {
    let mut event = LogEvent::new();

    // Pack metadata (timestamp + level + thread_id).
    event.packed_meta = LogEvent::pack_meta(timestamp_millis, level, self.thread_id);

    // Intern string identifiers (allocates only for new strings).
    event.target_id = self.interner.intern_target(target);
    event.message_id = self.interner.intern_message(message);

    event
  }

  pub fn build_with_fields(
    &mut self,
    timestamp_millis: u64,
    level: LogLevel,
    target: &str,
    message: &str,
    fields: &[(String, FieldValue)],
  ) -> LogEvent {
    let mut event = self.build_fast(timestamp_millis, level, target, message);

    for (key, value) in fields.iter().take(3) {
      let key_id = self.interner.intern_field(key);
      event.add_field(key_id, *value);
    }

    event
  }
  // #[inline]
  // pub fn build_from_tracing(&mut self, tracing_event: LogEvent) -> LogEvent {
  //   let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
  //
  //   self.build_fast(timestamp_millis, LogLevel::INFO, "", "")
  // }

  pub fn current_thread_id_u32() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    thread::current().id().hash(&mut hasher);
    hasher.finish() as u32
  }
}
