#[cfg(test)]
mod tests {
  use crate::{buffer::RingBuffer, event::Event, trace::Trace, trace_layer::BufferLayer};

  use std::{
    fs,
    sync::{Arc, Mutex},
  };
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
  fn test_flush_snapshot_creates_file() {
    // 1. Create a buffer and populate it with dummy events
    let buffer = Arc::new(Mutex::new(RingBuffer::<Event>::new(10)));

    {
      let mut buf_lock = buffer.lock().unwrap();
      // Add some dummy events
      buf_lock.push(Event::new(
        12345,
        "event1".to_string(),
        "message1".to_string(),
      ));
      buf_lock.push(Event::new(
        12345,
        "event2".to_string(),
        "message2".to_string(),
      ));
    }

    // 2. Call flush_snapshot
    Trace::flush_snapshot(buffer.clone(), "test");

    // 3. Check /tmp for a file that starts with "ttlog-<pid>-<timestamp>-test"
    let pid = std::process::id().to_string();
    let files: Vec<_> = fs::read_dir("/tmp")
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| {
        let name = e.file_name().to_string_lossy().to_string();
        name.contains(&pid) && name.contains("test") && name.ends_with(".bin")
      })
      .collect();

    assert!(!files.is_empty(), "Snapshot file should exist in /tmp");

    // Optional: remove created files after test
    for f in files {
      let _ = fs::remove_file(f.path());
    }
  }
}
