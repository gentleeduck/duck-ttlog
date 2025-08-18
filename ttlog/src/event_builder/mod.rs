mod __test__;
use std::{sync::Arc, thread};

use crate::{
  event::{FieldValue, LogEvent, LogLevel},
  string_interner::StringInterner,
};

#[derive(Debug)]
pub struct EventBuilder {
  interner: Arc<StringInterner>, // Stores/reuses strings like target/message names
  thread_id: u8,                 // Thread identifier for the log event
  event_pool: Vec<LogEvent>,     // Pre-allocated events for reuse
  pool_index: usize,             // Current position in the circular pool
}

impl EventBuilder {
  pub fn new(interner: Arc<StringInterner>) -> Self {
    let thread_id = current_thread_id_u64() as u8;

    Self {
      interner,
      thread_id,
      event_pool: Vec::with_capacity(16), // Pool of reusable events
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
    // Create a new event directly instead of using pool for simplicity
    let mut event = LogEvent::new();

    // Pack metadata efficiently
    event.packed_meta = LogEvent::pack_meta(timestamp_millis, level, self.thread_id);

    // Intern strings (only allocates for new strings)
    event.target_id = self.interner.intern_target(target);
    event.message_id = self.interner.intern_message(message);

    event
  }

  /// Build event with fields (slightly slower)
  pub fn build_with_fields(
    &mut self,
    timestamp_millis: u64,
    level: LogLevel,
    target: &str,
    message: &str,
    fields: &[(String, FieldValue)], // Changed from &[(&str, FieldValue)]
  ) -> LogEvent {
    let mut event = self.build_fast(timestamp_millis, level, target, message);

    // Add up to 3 fields
    for (key, value) in fields.iter().take(3) {
      let key_id = self.interner.intern_field(key);
      event.add_field(key_id, *value);
    }

    event
  }

  /// Build from tracing event
  #[inline]
  pub fn build_from_tracing(&mut self, tracing_event: &tracing::Event) -> LogEvent {
    let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
    let level = LogLevel::from_tracing_level(tracing_event.metadata().level());
    let target = tracing_event.metadata().target();

    // Extract message with zero-copy visitor
    let mut visitor = MessageVisitor::default();
    tracing_event.record(&mut visitor);
    let message = visitor.message.as_deref().unwrap_or("");

    self.build_fast(timestamp_millis, level, target, message)
  }
}

fn current_thread_id_u64() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}

/// Zero-copy message visitor - no allocations unless absolutely necessary
#[derive(Default)]
pub struct MessageVisitor {
  pub message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
  #[inline]
  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    }
  }

  #[inline]
  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    if field.name() == "message" && self.message.is_none() {
      self.message = Some(format!("{:?}", value));
    }
  }
}

// Thread-local builder for maximum performance
thread_local! {
    static BUILDER: std::cell::RefCell<Option<EventBuilder>> = std::cell::RefCell::new(None);
}

/// Global function to build events efficiently
pub fn build_event_fast(interner: Arc<StringInterner>, tracing_event: &tracing::Event) -> LogEvent {
  BUILDER.with(|builder_cell| {
    let mut builder_opt = builder_cell.borrow_mut();

    // Initialize thread-local builder if needed
    if builder_opt.is_none() {
      *builder_opt = Some(EventBuilder::new(interner));
    }

    builder_opt
      .as_mut()
      .unwrap()
      .build_from_tracing(tracing_event)
  })
}
