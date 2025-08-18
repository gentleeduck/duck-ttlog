use std::{sync::Arc, thread};

use crate::{event::LogEvent, string_interner::StringInterner};

pub struct EventBuilder {
  interner: Arc<StringInterner>,
  thread_id: u8,

  // Thread-local event pool to avoid allocations
  event_pool: Vec<LogEvent>,
  pool_index: usize,
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
}

fn current_thread_id_u64() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}
