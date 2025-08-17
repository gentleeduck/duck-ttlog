#[cfg(test)]
mod __test__ {
  use std::borrow::Cow;

  use crate::event::{EventBuilder, LogEvent, LogLevel};
  use crate::lf_buffer::LockFreeRingBuffer;
  use crate::snapshot::SnapshotWriter;

  #[test]
  fn test_snapshot_writer_new() {
    let writer = SnapshotWriter::new("test_service");
    assert_eq!(writer.service, "test_service");

    let writer = SnapshotWriter::new("another_service".to_string());
    assert_eq!(writer.service, "another_service");
  }

  #[test]
  fn test_snapshot_creation() {
    let buffer = LockFreeRingBuffer::new(5);
    let writer = SnapshotWriter::new("test_service");

    let mut builder = EventBuilder::new_with_capacity(0);
    // Add some events using the new builder API
    let event1 = builder
      .timestamp_nanos(1000)
      .level(LogLevel::Info)
      .target(Cow::Borrowed("target1"))
      .message(Cow::Borrowed("First"))
      .build();

    buffer.push_overwrite(event1);

    let event2 = EventBuilder::new_with_capacity(0)
      .timestamp_nanos(2000)
      .level(LogLevel::Warn)
      .target(Cow::Borrowed("target2"))
      .message(Cow::Borrowed("Second"))
      .build();

    buffer.push_overwrite(event2);

    let mut buffer_clone = buffer.clone();
    let snapshot = writer
      .create_snapshot(&mut buffer_clone, "test_reason")
      .unwrap();

    assert_eq!(snapshot.service, "test_service");
    assert_eq!(snapshot.reason, "test_reason");
    assert_eq!(snapshot.events.len(), 2);
    assert_eq!(snapshot.events[0].message, "First");
    assert_eq!(snapshot.events[1].message, "Second");

    // Buffer should be empty after snapshot
    assert!(buffer_clone.is_empty());
  }

  #[test]
  fn test_snapshot_creation_empty_buffer() {
    let buffer: LockFreeRingBuffer<LogEvent> = LockFreeRingBuffer::new(5);
    let writer = SnapshotWriter::new("test_service");

    let snapshot = writer.create_snapshot(&mut buffer.clone(), "empty_reason");

    assert!(snapshot.is_none());
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_snapshot_metadata() {
    let buffer = LockFreeRingBuffer::new(3);
    let writer = SnapshotWriter::new("metadata_test");

    let event = EventBuilder::new_with_capacity(0)
      .timestamp_nanos(1000)
      .level(LogLevel::Info)
      .target(Cow::Borrowed("target"))
      .message(Cow::Borrowed("Test"))
      .build();

    buffer.push_overwrite(event);

    let snapshot = writer
      .create_snapshot(&mut buffer.clone(), "metadata_test")
      .unwrap();

    assert_eq!(snapshot.service, "metadata_test");
    assert_eq!(snapshot.reason, "metadata_test");
    assert_eq!(snapshot.pid, std::process::id());
    assert!(!snapshot.hostname.is_empty());
    assert!(!snapshot.created_at.is_empty());

    // Check timestamp format (YYYYMMDDHHMMSS)
    assert_eq!(snapshot.created_at.len(), 14);
    assert!(snapshot.created_at.chars().all(|c| c.is_ascii_digit()));
  }
}
