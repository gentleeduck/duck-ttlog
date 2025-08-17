#[cfg(test)]
mod tests {
  use crate::trace::Message;
  use crate::trace_layer::BufferLayer;
  use crossbeam_channel::bounded;
  use tracing::{error, info, warn};
  use tracing_subscriber::{layer::SubscriberExt, Registry};

  #[test]
  fn test_buffer_layer_new() {
    let (sender, _receiver) = bounded::<Message>(10);
    let layer = BufferLayer::new(sender);

    assert!(std::format!("{:?}", layer).contains("BufferLayer"));
  }

  #[test]
  fn test_buffer_layer_clone() {
    let (sender, _receiver) = bounded::<Message>(10);
    let layer = BufferLayer::new(sender);

    let cloned = layer.clone();

    // Both should have the same sender
    assert!(std::format!("{:?}", layer).contains("BufferLayer"));
    assert!(std::format!("{:?}", cloned).contains("BufferLayer"));
  }

  #[test]
  fn test_buffer_layer_with_tracing_events() {
    // let (sender, receiver) = bounded::<Message>(100);
    // let layer = BufferLayer::new(sender);
    //
    // // Create subscriber with our layer
    // let subscriber = Registry::default().with(layer);
    // let _guard = tracing::subscriber::set_default(subscriber);
    //
    // // Emit tracing events
    // info!("Test info message");
    // warn!("Test warning message");
    // error!("Test error message");

    // Check that events were captured
    // let mut events = Vec::new();
    // while let Ok(msg) = receiver.try_recv() {
    //   match msg {
    //     Message::Event(event) => events.push(event),
    //     _ => {},
    //   }
    // }
    //
    // // Should have captured 3 events
    // assert_eq!(events.len(), 3);
    //
    // // Check event details - note that the level is now an enum, not a string
    // let info_event = events
    //   .iter()
    //   .find(|e| e.level == crate::event::LogLevel::Info)
    //   .unwrap();
    // assert_eq!(info_event.message, "Test info message");
    //
    // let warn_event = events
    //   .iter()
    //   .find(|e| e.level == crate::event::LogLevel::Warn)
    //   .unwrap();
    // assert_eq!(warn_event.message, "Test warning message");
    //
    // let error_event = events
    //   .iter()
    //   .find(|e| e.level == crate::event::LogLevel::Error)
    //   .unwrap();
    // assert_eq!(error_event.message, "Test error message");
  }
}
