mod __test__;
use std::sync::{Arc, Mutex};

use crate::{buffer::RingBuffer, event::Event};
use chrono::Utc;
use tracing::{field::Field, Event as TracingEvent, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Debug, Clone)]
pub struct BufferLayer {
  pub buffer: Arc<Mutex<RingBuffer<Event>>>,
}

impl BufferLayer {
  pub fn new(buffer: Arc<Mutex<RingBuffer<Event>>>) -> Self {
    Self { buffer }
  }
}

impl<T> Layer<T> for BufferLayer
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    let ts = Utc::now().timestamp_millis() as u64;
    let level = event.metadata().level().to_string();

    // Extract message
    let mut visitor = MessageVisitor::default();
    event.record(&mut visitor);
    let message = visitor.message.unwrap_or_else(|| "".to_string());

    let new_event = Event::new(ts, level, message);

    if let Ok(mut buf) = self.buffer.lock() {
      buf.push(new_event);
    }
  }
}

#[derive(Default)]
struct MessageVisitor {
  message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
  fn record_str(&mut self, _field: &Field, value: &str) {
    self.message = Some(value.to_string());
  }

  fn record_debug(&mut self, _field: &Field, value: &dyn std::fmt::Debug) {
    self.message = Some(format!("{:?}", value));
  }
}
