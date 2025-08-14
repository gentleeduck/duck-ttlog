#[cfg(test)]
mod __test__ {
  use crate::buffer::RingBuffer;
  use crate::event::Event;
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
    let mut buffer = RingBuffer::new(5);
    let writer = SnapshotWriter::new("test_service");

    // Add some events
    buffer.push(Event::new(
      1000,
      "INFO".to_string(),
      "First".to_string(),
      "target1".to_string(),
    ));
    buffer.push(Event::new(
      2000,
      "WARN".to_string(),
      "Second".to_string(),
      "target2".to_string(),
    ));

    let snapshot = writer.create_snapshot(&mut buffer, "test_reason").unwrap();

    assert_eq!(snapshot.service, "test_service");
    assert_eq!(snapshot.reason, "test_reason");
    assert_eq!(snapshot.events.len(), 2);
    assert_eq!(snapshot.events[0].message, "First");
    assert_eq!(snapshot.events[1].message, "Second");

    // Buffer should be empty after snapshot
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_snapshot_creation_empty_buffer() {
    let mut buffer: RingBuffer<Event> = RingBuffer::new(5);
    let writer = SnapshotWriter::new("test_service");

    let snapshot = writer.create_snapshot(&mut buffer, "empty_reason");

    assert!(snapshot.is_none());
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_snapshot_metadata() {
    let mut buffer = RingBuffer::new(3);
    let writer = SnapshotWriter::new("metadata_test");

    buffer.push(Event::new(
      1000,
      "INFO".to_string(),
      "Test".to_string(),
      "target".to_string(),
    ));

    let snapshot = writer
      .create_snapshot(&mut buffer, "metadata_test")
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
