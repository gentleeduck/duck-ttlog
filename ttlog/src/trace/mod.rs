mod __test__;

use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

use crate::buffer::RingBuffer;
use crate::event::Event;
use crate::trace_layer::BufferLayer;

pub struct Trace {
  buffer: Arc<Mutex<RingBuffer<Event>>>,
}

impl Trace {
  pub fn init(capacity: usize) -> Self {
    let buffer = Arc::new(Mutex::new(RingBuffer::new(capacity)));
    let layer = BufferLayer::new(buffer.clone());

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber)
      .expect("Failed to set global tracing subscriber");

    Self { buffer }
  }

  pub fn print_logs(&self) {
    let buf = self.buffer.lock().unwrap();
    for event in buf.iter() {
      println!("[{}] {} - {}", event.timestamps, event.level, event.message);
    }
  }
}
