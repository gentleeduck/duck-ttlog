mod __test__;

use std::{cell::RefCell, sync::Arc};

use crate::{
  event::{FieldValue, LogLevel},
  event_builder::{EventBuilder, MessageVisitor},
  string_interner::StringInterner,
  trace::Message,
};

use crossbeam_channel::{Sender, TrySendError};
use tracing::{Event as TracingEvent, Subscriber};
use tracing_subscriber::{layer::Context, registry::LookupSpan, Layer};

#[derive(Debug, Clone)]
pub struct BufferLayer {
  /// Channel sender used to forward captured events.
  sender: Sender<Message>,
  /// String interner for efficient string storage.
  interner: Arc<StringInterner>,
}

impl BufferLayer {
  /// Creates a new `BufferLayer` that will send events to the given channel.
  pub fn new(sender: Sender<Message>, interner: Arc<StringInterner>) -> Self {
    Self { sender, interner }
  }
}
// Thread-local event builder for zero-allocation event creation
thread_local! {
    static LAYER_BUILDER: RefCell<Option<EventBuilder>> = RefCell::new(None);
}

impl<T> Layer<T> for BufferLayer
where
  T: Subscriber + for<'a> LookupSpan<'a>,
{
  fn on_event(&self, event: &TracingEvent<'_>, _ctx: Context<'_, T>) {
    let interner = Arc::clone(&self.interner);

    // Use thread-local builder for maximum performance
    LAYER_BUILDER.with(|builder_cell| {
      let mut builder_opt = builder_cell.borrow_mut();

      // Initialize thread-local builder if needed
      if builder_opt.is_none() {
        *builder_opt = Some(EventBuilder::new(Arc::clone(&interner)));
      }

      let builder = builder_opt.as_mut().unwrap();

      // Extract all event data
      let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
      let level = LogLevel::from_tracing_level(event.metadata().level());
      let target = event.metadata().target();

      // Extract message and fields using comprehensive visitor
      // let mut visitor = ComprehensiveVisitor::new(Arc::clone(&interner));
      // event.record(&mut visitor);
      //
      // let message = visitor.message.as_deref().unwrap_or("");

      let mut visitor = MessageVisitor::default();
      event.record(&mut visitor);
      let message = visitor.message.as_deref().unwrap_or("");
      let log_event = builder.build_fast(timestamp_millis, level, target, message);

      // Build the complete event with all extracted data
      // let log_event = builder.build_fast(timestamp_millis, level, target, message);
      // let log_event = if visitor.fields.is_empty() {
      // } else {
      //   builder.build_with_fields(timestamp_millis, level, target, message, &visitor.fields)
      // };

      // Attempt non-blocking send
      match self.sender.try_send(Message::Event(log_event)) {
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
    });
  }
}

/// Comprehensive visitor that extracts message and all fields from tracing events
pub struct ComprehensiveVisitor {
  pub message: Option<String>,
  pub fields: Vec<(&'static str, FieldValue)>,
  interner: Arc<StringInterner>,
}

impl ComprehensiveVisitor {
  pub fn new(interner: Arc<StringInterner>) -> Self {
    Self {
      message: None,
      fields: Vec::new(),
      interner,
    }
  }
}

impl tracing::field::Visit for ComprehensiveVisitor {
  fn record_f64(&mut self, field: &tracing::field::Field, value: f64) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      self.fields.push((field.name(), FieldValue::F64(value)));
    }
  }

  fn record_i64(&mut self, field: &tracing::field::Field, value: i64) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      self.fields.push((field.name(), FieldValue::I64(value)));
    }
  }

  fn record_u64(&mut self, field: &tracing::field::Field, value: u64) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      self.fields.push((field.name(), FieldValue::U64(value)));
    }
  }

  fn record_bool(&mut self, field: &tracing::field::Field, value: bool) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      self.fields.push((field.name(), FieldValue::Bool(value)));
    }
  }

  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    } else {
      // Intern the string and store as StringId
      let string_id = self.interner.intern_field(value);
      self
        .fields
        .push((field.name(), FieldValue::StringId(string_id)));
    }
  }

  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    let debug_str = format!("{:?}", value);
    if field.name() == "message" {
      self.message = Some(debug_str);
    } else {
      // Intern the debug string
      let string_id = self.interner.intern_field(&debug_str);
      self
        .fields
        .push((field.name(), FieldValue::StringId(string_id)));
    }
  }
}
