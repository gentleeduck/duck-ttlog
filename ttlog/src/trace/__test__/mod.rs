#[cfg(test)]
mod tests {
  use crate::{buffer::RingBuffer, trace::Trace, trace_layer::BufferLayer};

  use std::sync::{Arc, Mutex};
  use tracing::{info, subscriber::with_default};
  use tracing_subscriber::{layer::SubscriberExt, Registry};

  /// Helper to initialize Trace locally for tests
  fn init_local_trace(capacity: usize) -> Trace {
    let buffer = Arc::new(Mutex::new(RingBuffer::new(capacity)));
    let layer = BufferLayer::new(buffer.clone());

    let subscriber = Registry::default().with(layer);

    // Activate subscriber for this thread only
    with_default(subscriber, || {
      // Tracing macros will log to this layer in this scope
    });

    Trace { buffer }
  }

  #[test]
  fn trace_init_and_log() {
    let trace = init_local_trace(5);

    // Log some events inside a local subscriber scope
    with_default(
      Registry::default().with(BufferLayer::new(trace.buffer.clone())),
      || {
        info!("First event");
        info!("Second event");
      },
    );

    let buf = trace.buffer.lock().unwrap();
    assert_eq!(buf.iter().collect::<Vec<_>>().len(), 2);

    let messages: Vec<String> = buf.iter().map(|e| e.message.clone()).collect();
    assert!(messages.contains(&"First event".to_string()));
    assert!(messages.contains(&"Second event".to_string()));
  }

  #[test]
  fn trace_buffer_wraparound() {
    let trace = init_local_trace(3);

    with_default(
      Registry::default().with(BufferLayer::new(trace.buffer.clone())),
      || {
        for i in 0..5 {
          info!("Event {}", i);
        }
      },
    );

    let buf = trace.buffer.lock().unwrap();
    assert_eq!(buf.iter().collect::<Vec<_>>().len(), 3); // Only last 3 events remain

    let messages: Vec<String> = buf.iter().map(|e| e.message.clone()).collect();
    assert_eq!(messages, vec!["Event 2", "Event 3", "Event 4"]);
  }

  #[test]
  fn trace_print_logs() {
    let trace = init_local_trace(2);

    with_default(
      Registry::default().with(BufferLayer::new(trace.buffer.clone())),
      || {
        info!("Hello");
        info!("World");
      },
    );

    // Ensure print_logs runs without panic
    trace.print_logs();
  }
}
