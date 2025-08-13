use std::sync::{Arc, Mutex};

use crate::{buffer::TTlogBuffer, event::Event, trace::BufferLayer};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

pub mod buffer;
pub mod event;
pub mod trace;

pub fn init(capacity: usize) -> Arc<Mutex<TTlogBuffer<Event>>> {
  let buffer = Arc::new(Mutex::new(TTlogBuffer::new(capacity)));
  let layer = BufferLayer::new(buffer.clone());

  tracing_subscriber::registry().with(layer).init();

  buffer
}
