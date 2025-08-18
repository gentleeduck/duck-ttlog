use crate::{
  event::{LogEvent, LogLevel},
  event_builder::{build_event_stack, EventBuilder, MessageVisitor},
  string_interner::StringInterner,
  trace::Message,
};
use crossbeam_channel::{Sender, TrySendError};
use std::{cell::UnsafeCell, sync::Arc};
use tracing::{Event as TracingEvent, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Debug, Clone)]
pub struct BufferLayer {
  sender: Sender<Message>,
  interner: Arc<StringInterner>,
}

impl BufferLayer {
  pub fn new(sender: Sender<Message>, interner: Arc<StringInterner>) -> Self {
    Self { sender, interner }
  }
}

// Ultra-fast thread-local builder using UnsafeCell
thread_local! {
    static LAYER_BUILDER: UnsafeCell<Option<EventBuilder>> = UnsafeCell::new(None);
}

impl<T> Layer<T> for BufferLayer
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  #[inline]
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    let interner = &self.interner; // Avoid Arc clone on hot path

    // Fast path: use stack-based building when possible
    let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
    let level = LogLevel::from_tracing_level(event.metadata().level());
    let target = event.metadata().target();

    // Extract message efficiently
    let mut visitor = MessageVisitor::default();
    event.record(&mut visitor);
    let message = visitor.message.as_deref().unwrap_or("");

    // Use the fastest building method
    let log_event = build_event_stack(interner, timestamp_millis, level, target, message);

    // Non-blocking send with minimal error handling
    if let Err(TrySendError::Disconnected(_)) = self.sender.try_send(Message::Event(log_event)) {
      // Only handle disconnect - ignore full channel
      eprintln!("[BufferLayer] Writer disconnected");
    }
  }
}

// Alternative implementation using thread-local builder for comparison
pub struct BufferLayerWithBuilder {
  sender: Sender<Message>,
  interner: Arc<StringInterner>,
}

impl BufferLayerWithBuilder {
  pub fn new(sender: Sender<Message>, interner: Arc<StringInterner>) -> Self {
    Self { sender, interner }
  }
}

impl<T> Layer<T> for BufferLayerWithBuilder
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  #[inline]
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    let interner = Arc::clone(&self.interner);

    LAYER_BUILDER.with(|builder_cell| {
      let builder_ptr = builder_cell.get();

      unsafe {
        // Initialize if needed
        if (*builder_ptr).is_none() {
          *builder_ptr = Some(EventBuilder::new(interner));
        }

        // Build event
        let log_event = (*builder_ptr).as_mut().unwrap().build_from_tracing(event);

        // Send without blocking
        let _ = self.sender.try_send(Message::Event(log_event));
      }
    });
  }
}

// Batch processing layer for even higher throughput
const BATCH_SIZE: usize = 32;

thread_local! {
    static EVENT_BATCH: UnsafeCell<Vec<LogEvent>> = UnsafeCell::new(Vec::with_capacity(BATCH_SIZE));
}

pub struct BatchedBufferLayer {
  sender: Sender<Message>,
  interner: Arc<StringInterner>,
}

impl BatchedBufferLayer {
  pub fn new(sender: Sender<Message>, interner: Arc<StringInterner>) -> Self {
    Self { sender, interner }
  }

  #[cold]
  fn flush_batch(sender: &Sender<Message>, batch: &mut Vec<LogEvent>) {
    if !batch.is_empty() {
      // Try to send all events in batch
      for event in batch.drain(..) {
        let _ = sender.try_send(Message::Event(event));
      }
    }
  }
}

impl<T> Layer<T> for BatchedBufferLayer
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  #[inline]
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    EVENT_BATCH.with(|batch_cell| {
      let batch_ptr = batch_cell.get();

      unsafe {
        let batch = &mut *batch_ptr;

        // Build event directly into batch
        let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
        let level = LogLevel::from_tracing_level(event.metadata().level());
        let target = event.metadata().target();

        let mut visitor = MessageVisitor::default();
        event.record(&mut visitor);
        let message = visitor.message.as_deref().unwrap_or("");

        let log_event = build_event_stack(&self.interner, timestamp_millis, level, target, message);
        batch.push(log_event);

        // Flush when batch is full
        if batch.len() >= BATCH_SIZE {
          Self::flush_batch(&self.sender, batch);
        }
      }
    });
  }
}
