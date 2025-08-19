#[cfg(test)]
mod __test__ {

  use crate::event::{FieldValue, LogEvent, LogLevel};
  use crate::event_builder::EventBuilder;
  use crate::lf_buffer::LockFreeRingBuffer;
  use crate::snapshot::{Snapshot, SnapshotWriter};
  use crate::string_interner::StringInterner;
  use std::sync::Arc;

  #[test]
  fn test_snapshot_writer_new() {
    SnapshotWriter::new("test_service");
    // SnapshotWriter doesn't expose fields directly, so we test through usage
    assert!(true); // Constructor should not panic
  }

  #[test]
  fn test_create_snapshot_empty_ring() {
    let writer = SnapshotWriter::new("test_service");
    let mut ring = LockFreeRingBuffer::<LogEvent>::new(10);

    let snapshot = writer.create_snapshot(&mut ring, "test_reason");
    assert!(snapshot.is_none()); // Empty ring should return None
  }

  #[test]
  fn test_create_snapshot_with_events() {
    let writer = SnapshotWriter::new("test_service");
    let mut ring = LockFreeRingBuffer::<LogEvent>::new(10);

    // Create some test events
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner);

    let event1 = builder.build_fast(1000, LogLevel::INFO, "module1", "message1");
    let event2 = builder.build_fast(2000, LogLevel::ERROR, "module2", "message2");

    // Add events to ring buffer
    ring.push(event1).unwrap();
    ring.push(event2).unwrap();

    let snapshot = writer.create_snapshot(&mut ring, "test_reason").unwrap();

    // Verify snapshot structure
    assert_eq!(snapshot.service, "test_service");
    assert_eq!(snapshot.reason, "test_reason");
    assert_eq!(snapshot.events.len(), 2);
    assert!(!snapshot.hostname.is_empty());
    assert!(snapshot.pid > 0);
    assert!(!snapshot.created_at.is_empty());

    // Verify events
    assert_eq!(snapshot.events[0].timestamp_millis(), 1000);
    assert_eq!(snapshot.events[0].level(), LogLevel::INFO);
    assert_eq!(snapshot.events[1].timestamp_millis(), 2000);
    assert_eq!(snapshot.events[1].level(), LogLevel::ERROR);
  }

  #[test]
  fn test_snapshot_serialization() {
    let snapshot = Snapshot {
      service: "test_service".to_string(),
      hostname: "test_host".to_string(),
      pid: 12345,
      created_at: "20240101120000".to_string(),
      reason: "test_reason".to_string(),
      events: vec![],
    };

    // Test JSON serialization
    let json = serde_json::to_string(&snapshot).expect("Failed to serialize");
    let deserialized: Snapshot = serde_json::from_str(&json).expect("Failed to deserialize");

    assert_eq!(deserialized.service, snapshot.service);
    assert_eq!(deserialized.hostname, snapshot.hostname);
    assert_eq!(deserialized.pid, snapshot.pid);
    assert_eq!(deserialized.created_at, snapshot.created_at);
    assert_eq!(deserialized.reason, snapshot.reason);
    assert_eq!(deserialized.events.len(), snapshot.events.len());
  }

  #[test]
  fn test_snapshot_with_fields() {
    let writer = SnapshotWriter::new("field_test_service");
    let mut ring = LockFreeRingBuffer::<LogEvent>::new(10);

    // Create event with fields
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner);

    let fields = vec![
      ("user_id".to_string(), FieldValue::U64(12345)),
      ("error_code".to_string(), FieldValue::I32(-1)),
      ("success".to_string(), FieldValue::Bool(false)),
    ];

    let event = builder.build_with_fields(
      3000,
      LogLevel::WARN,
      "auth_module",
      "Authentication failed",
      &fields,
    );

    ring.push(event).unwrap();

    let snapshot = writer.create_snapshot(&mut ring, "auth_failure").unwrap();

    assert_eq!(snapshot.events.len(), 1);
    assert_eq!(snapshot.events[0].field_count, 3);
    assert_eq!(snapshot.reason, "auth_failure");
  }

  #[test]
  fn test_snapshot_clone() {
    let original = Snapshot {
      service: "clone_test".to_string(),
      hostname: "host1".to_string(),
      pid: 999,
      created_at: "20240101000000".to_string(),
      reason: "clone_reason".to_string(),
      events: vec![],
    };

    let cloned = original.clone();

    assert_eq!(cloned.service, original.service);
    assert_eq!(cloned.hostname, original.hostname);
    assert_eq!(cloned.pid, original.pid);
    assert_eq!(cloned.created_at, original.created_at);
    assert_eq!(cloned.reason, original.reason);
    assert_eq!(cloned.events.len(), original.events.len());
  }

  #[test]
  fn test_snapshot_debug_format() {
    let snapshot = Snapshot {
      service: "debug_test".to_string(),
      hostname: "debug_host".to_string(),
      pid: 777,
      created_at: "20240101111111".to_string(),
      reason: "debug_reason".to_string(),
      events: vec![],
    };

    let debug_str = format!("{:?}", snapshot);
    assert!(debug_str.contains("debug_test"));
    assert!(debug_str.contains("debug_host"));
    assert!(debug_str.contains("777"));
    assert!(debug_str.contains("debug_reason"));
  }

  #[test]
  fn test_multiple_snapshots_same_writer() {
    let writer = SnapshotWriter::new("multi_snapshot_service");
    let mut ring1 = LockFreeRingBuffer::<LogEvent>::new(5);
    let mut ring2 = LockFreeRingBuffer::<LogEvent>::new(5);

    // Create events for first ring
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner);

    let event1 = builder.build_fast(1000, LogLevel::INFO, "module1", "first");
    ring1.push(event1).unwrap();

    let event2 = builder.build_fast(2000, LogLevel::ERROR, "module2", "second");
    ring2.push(event2).unwrap();

    // Create snapshots
    let snapshot1 = writer.create_snapshot(&mut ring1, "reason1").unwrap();
    let snapshot2 = writer.create_snapshot(&mut ring2, "reason2").unwrap();

    // Both should have same service but different reasons
    assert_eq!(snapshot1.service, "multi_snapshot_service");
    assert_eq!(snapshot2.service, "multi_snapshot_service");
    assert_eq!(snapshot1.reason, "reason1");
    assert_eq!(snapshot2.reason, "reason2");
    assert_eq!(snapshot1.events.len(), 1);
    assert_eq!(snapshot2.events.len(), 1);
  }

  #[test]
  fn test_snapshot_large_event_count() {
    let writer = SnapshotWriter::new("large_test_service");
    let mut ring = LockFreeRingBuffer::<LogEvent>::new(1000);

    // Create many events
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner);

    for i in 0..500 {
      let event = builder.build_fast(
        i as u64,
        LogLevel::DEBUG,
        "test_module",
        &format!("message_{}", i),
      );
      ring.push(event).unwrap();
    }

    let snapshot = writer.create_snapshot(&mut ring, "large_snapshot").unwrap();

    assert_eq!(snapshot.events.len(), 500);
    assert_eq!(snapshot.reason, "large_snapshot");

    // Verify first and last events
    assert_eq!(snapshot.events[0].timestamp_millis(), 0);
    assert_eq!(snapshot.events[499].timestamp_millis(), 499);
  }
}
