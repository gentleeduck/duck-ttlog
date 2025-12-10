#[cfg(test)]
mod __test__ {

  use std::sync::Arc;

  use crate::event::{FieldValue, LogEvent, LogLevel};
  use crate::event_builder::EventBuilder;
  use crate::lf_buffer::LockFreeRingBuffer;
  use crate::snapshot::{SnapShot, SnapshotWriter};
  use crate::string_interner::StringInterner;

  fn builder_with_ring(
    capacity: usize,
  ) -> (
    Arc<LockFreeRingBuffer<LogEvent>>,
    Arc<StringInterner>,
    EventBuilder,
  ) {
    let interner = Arc::new(StringInterner::new());
    let builder = EventBuilder::new(interner.clone());
    let ring = LockFreeRingBuffer::new_shared(capacity);
    (ring, interner, builder)
  }

  #[test]
  fn test_snapshot_writer_new() {
    SnapshotWriter::new("test_service");
  }

  #[test]
  fn test_create_snapshot_empty_ring() {
    let writer = SnapshotWriter::new("test_service");
    let (mut ring, interner, _) = builder_with_ring(10);

    let snapshot = writer.create_snapshot(&mut ring, "test_reason", interner);
    assert!(snapshot.is_none());
  }

  #[test]
  fn test_create_snapshot_with_events() {
    let writer = SnapshotWriter::new("test_service");
    let (mut ring, interner, builder) = builder_with_ring(10);

    let event1 = builder.build_fast(1000, LogLevel::INFO, "module1", "message1");
    let event2 = builder.build_fast(2000, LogLevel::ERROR, "module2", "message2");
    ring.push(event1).unwrap();
    ring.push(event2).unwrap();

    let snapshot = writer
      .create_snapshot(&mut ring, "test_reason", interner.clone())
      .unwrap();

    assert_eq!(snapshot.service, "test_service");
    assert_eq!(snapshot.reason, "test_reason");
    assert_eq!(snapshot.events.len(), 2);
    assert!(snapshot.pid > 0);
    assert!(!snapshot.hostname.is_empty());

    assert_eq!(snapshot.events[0].timestamp_millis(), 1000);
    assert_eq!(snapshot.events[0].level(), LogLevel::INFO);
    assert_eq!(snapshot.events[1].timestamp_millis(), 2000);
    assert_eq!(snapshot.events[1].level(), LogLevel::ERROR);

    assert_eq!(snapshot.events[0].target, "module1");
  }

  #[test]
  fn test_snapshot_serialization() {
    let snapshot = SnapShot {
      service: "test_service".to_string(),
      hostname: "test_host".to_string(),
      pid: 12345,
      created_at: "20240101120000".to_string(),
      reason: "test_reason".to_string(),
      events: vec![],
    };

    let json = serde_json::to_string(&snapshot).expect("serialize");
    let deserialized: SnapShot = serde_json::from_str(&json).expect("deserialize");

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
    let (mut ring, interner, builder) = builder_with_ring(10);

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

    let snapshot = writer
      .create_snapshot(&mut ring, "auth_failure", interner)
      .unwrap();

    assert_eq!(snapshot.events.len(), 1);
    assert_eq!(snapshot.reason, "auth_failure");
    let kv = &snapshot.events[0].kv;
    assert_eq!(kv["user_id"], serde_json::json!(12345));
    assert_eq!(kv["error_code"], serde_json::json!(-1));
    assert_eq!(kv["success"], serde_json::json!(false));
  }

  #[test]
  fn test_snapshot_clone() {
    let original = SnapShot {
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
    let snapshot = SnapShot {
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
    let (mut ring1, interner1, builder1) = builder_with_ring(5);
    let (mut ring2, interner2, builder2) = builder_with_ring(5);

    ring1
      .push(builder1.build_fast(1000, LogLevel::INFO, "module1", "first"))
      .unwrap();
    ring2
      .push(builder2.build_fast(2000, LogLevel::ERROR, "module2", "second"))
      .unwrap();

    let snapshot1 = writer
      .create_snapshot(&mut ring1, "reason1", interner1)
      .unwrap();
    let snapshot2 = writer
      .create_snapshot(&mut ring2, "reason2", interner2)
      .unwrap();

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
    let (mut ring, interner, builder) = builder_with_ring(1000);

    for i in 0..500 {
      let event = builder.build_fast(
        i as u64,
        LogLevel::DEBUG,
        "test_module",
        &format!("message_{}", i),
      );
      ring.push(event).unwrap();
    }

    let snapshot = writer
      .create_snapshot(&mut ring, "large_snapshot", interner)
      .unwrap();

    assert_eq!(snapshot.events.len(), 500);
    assert_eq!(snapshot.reason, "large_snapshot");
    assert_eq!(snapshot.events[0].timestamp_millis(), 0);
    assert_eq!(snapshot.events[499].timestamp_millis(), 499);
  }
}
