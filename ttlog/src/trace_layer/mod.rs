mod __test__;

use crate::event::{EventBuilder, LogLevel};
use crate::trace::Message;

use chrono::Utc;
use crossbeam_channel::{Sender, TrySendError};
use tracing::field::Visit;
use tracing::{field::Field, Event as TracingEvent, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

/// `BufferLayer` is a `tracing` layer that captures tracing events and
/// forwards them to a channel for asynchronous processing.
///
/// This layer converts a `tracing::Event` into a minimal `Event` struct
/// containing only the timestamp, log level, and message, and sends it
/// to a `crossbeam_channel::Sender<Message>`.
///
/// # Design
/// - Non-blocking: uses `try_send` to avoid slowing down the tracing hot path.
/// - Drops events if the channel is full to prevent blocking.
/// - Handles disconnected channels gracefully.
#[derive(Debug, Clone)]
pub struct BufferLayer {
  /// Channel sender used to forward captured events.
  sender: Sender<Message>,
}

impl BufferLayer {
  /// Creates a new `BufferLayer` that will send events to the given channel.
  ///
  /// # Parameters
  /// - `sender`: A `crossbeam_channel::Sender<Message>` to forward captured events.
  pub fn new(sender: Sender<Message>) -> Self {
    Self { sender }
  }
}

impl<T> Layer<T> for BufferLayer
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  /// Called for every tracing event.
  ///
  /// Converts the event into a minimal `Event` (timestamp + level + message)
  /// and attempts to send it through the channel. Drops the event if the
  /// channel is full, or logs an error if the channel is disconnected.
  ///
  /// # Parameters
  /// - `event`: The `tracing::Event` being recorded.
  /// - `_ctx`: The subscriber context (unused in this implementation).
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    // Capture timestamp and level
    let ts = Utc::now().timestamp_millis() as u64;
    let level = LogLevel::get_typo(event.metadata().level().as_str());

    // Extract the message field using a visitor
    let mut visitor = MessageVisitor::default();
    event.record(&mut visitor);
    let message = visitor.message.unwrap_or_else(|| "".to_string());
    let target = event.metadata().target();

    // Build a minimal Event
    let new_event = EventBuilder::new_with_capacity(2)
      .timestamp_nanos(ts)
      .message(message)
      .target(target)
      .level(level)
      .build();

    // Attempt non-blocking send; drop if channel full
    match self.sender.try_send(Message::Event(new_event)) {
      Ok(_) => {},
      Err(err) => match err {
        TrySendError::Full(_) => {
          // Optional: increment a dropped-events counter here
        },
        TrySendError::Disconnected(_) => {
          // Writer thread died; log error
          eprintln!("[BufferLayer] writer thread disconnected");
        },
      },
    }
  }
}

/// `MessageVisitor` is a helper struct used to extract a string message
/// from structured tracing fields.
///
/// This is typically used when subscribing to tracing events and you want
/// to capture a specific field (like a message) from the event in a uniform way.
#[derive(Default)]
struct MessageVisitor {
  /// Stores the captured message from the tracing field.
  pub message: Option<String>,
}

impl Visit for MessageVisitor {
  /// Records a string field from a tracing event.
  ///
  /// # Parameters
  /// - `_field`: The `Field` metadata (ignored in this implementation).
  /// - `value`: The string value to record.
  ///
  /// # Behavior
  /// Stores the string value in the `message` field, replacing any previous value.
  fn record_str(&mut self, _field: &Field, value: &str) {
    self.message = Some(value.to_string());
  }

  /// Records a field that implements the `Debug` trait.
  ///
  /// # Parameters
  /// - `_field`: The `Field` metadata (ignored in this implementation).
  /// - `value`: The value to record, formatted using `Debug`.
  ///
  /// # Behavior
  /// Converts the value to a string using `format!("{:?}", value)` and stores it
  /// in the `message` field, replacing any previous value.
  fn record_debug(&mut self, _field: &Field, value: &dyn std::fmt::Debug) {
    self.message = Some(format!("{:?}", value));
  }
}
