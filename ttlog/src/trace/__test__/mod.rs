#[cfg(test)]
mod __test__ {

  use crate::event::Event;
  use crate::trace::{Message, Trace};

  #[test]
  fn test_trace_init() {
    let trace_system = Trace::init(100, 50);

    // Should have a sender
    assert!(trace_system.sender.capacity().unwrap_or(0) >= 50);
  }

  #[test]
  fn test_trace_get_sender() {
    let trace_system = Trace::init(100, 50);
    let sender = trace_system.get_sender();

    // Should be able to send messages
    let result = sender.send(Message::Event(Event::new(
      1000,
      "INFO".to_string(),
      "Test message".to_string(),
      "test_target".to_string(),
    )));

    assert!(result.is_ok());
  }

  #[test]
  fn test_trace_request_snapshot() {
    let trace_system = Trace::init(100, 50);

    // Should not panic
    trace_system.request_snapshot("test_snapshot");
  }

  #[test]
  fn test_trace_message_display() {
    let event = Event::new(
      1000,
      "INFO".to_string(),
      "Test message".to_string(),
      "test_target".to_string(),
    );

    let messages = vec![
      Message::Event(event.clone()),
      Message::SnapshotImmediate("test_reason".to_string()),
      Message::FlushAndExit,
    ];

    for msg in messages {
      let display_str = format!("{}", msg);
      assert!(!display_str.is_empty());
    }
  }

  #[test]
  fn test_trace_with_tracing_integration() {
    let _trace_system = Trace::init(100, 50);

    // Generate some tracing events
    tracing::info!("Integration test info");
    tracing::warn!("Integration test warning");
    tracing::error!("Integration test error");

    // Test should complete without crashing
  }
}
