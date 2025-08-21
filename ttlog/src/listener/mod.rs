use crate::event::LogEvent;
use crate::string_interner::StringInterner;

pub trait LogFormatter: Send + Sync {
  fn format(&self, event: &LogEvent, interner: &StringInterner, out: &mut String);
}

/// Core trait for log event listeners.
/// Designed for maximum performance - no error returns, no async.
pub trait LogListener: Send + Sync + 'static {
  /// Handle a single log event.
  /// Panics are caught at the call site to isolate listener failures.
  fn handle(&self, event: &LogEvent, interner: &StringInterner);

  /// Optional: Handle events in batch for better performance.
  /// Default implementation calls handle() for each event.
  fn handle_batch(&self, events: &[LogEvent], interner: &StringInterner) {
    for event in events {
      self.handle(event, interner);
    }
  }

  /// Optional: Called when listener is added (setup resources)
  fn on_start(&self) {}

  /// Optional: Called during shutdown (cleanup resources)  
  fn on_shutdown(&self) {}
}
