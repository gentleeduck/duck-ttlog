#[cfg(test)]
mod __test__ {

  use std::num::NonZeroU16;
  use std::sync::Arc;

  use crate::event::{Field, FieldValue, LogEvent, LogLevel};
  use crate::event_builder::EventBuilder;
  use crate::string_interner::StringInterner;

  #[test]
  fn test_log_level_from_str() {
    assert_eq!(LogLevel::from_str("trace"), LogLevel::TRACE);
    assert_eq!(LogLevel::from_str("DEBUG"), LogLevel::DEBUG);
    assert_eq!(LogLevel::from_str("Info"), LogLevel::INFO);
    assert_eq!(LogLevel::from_str("warn"), LogLevel::WARN);
    assert_eq!(LogLevel::from_str("error"), LogLevel::ERROR);
    assert_eq!(LogLevel::from_str("fatal"), LogLevel::FATAL);
    assert_eq!(LogLevel::from_str("unknown"), LogLevel::INFO);
  }

  #[test]
  fn test_log_level_from_tracing_level() {
    assert_eq!(
      LogLevel::from_tracing_level(&tracing::Level::TRACE),
      LogLevel::TRACE
    );
    assert_eq!(
      LogLevel::from_tracing_level(&tracing::Level::DEBUG),
      LogLevel::DEBUG
    );
    assert_eq!(
      LogLevel::from_tracing_level(&tracing::Level::INFO),
      LogLevel::INFO
    );
    assert_eq!(
      LogLevel::from_tracing_level(&tracing::Level::WARN),
      LogLevel::WARN
    );
    assert_eq!(
      LogLevel::from_tracing_level(&tracing::Level::ERROR),
      LogLevel::ERROR
    );
  }

  #[test]
  fn test_log_event_new() {
    let event = LogEvent::new();
    assert_eq!(event.packed_meta, 0);
    assert_eq!(event.target_id, 0);
    assert!(event.message_id.is_none());
    assert!(event.kv_id.is_none());
    assert_eq!(event.file_id, 0);
    assert_eq!(event.position, (0, 0));
  }

  #[test]
  fn test_log_event_pack_unpack_meta() {
    let timestamp = 1234567890;
    let level = LogLevel::ERROR;
    let thread_id = 42;

    let packed = LogEvent::pack_meta(timestamp, level, thread_id);
    let (unpacked_timestamp, unpacked_level, unpacked_thread_id) = LogEvent::unpack_meta(packed);

    assert_eq!(unpacked_timestamp, timestamp);
    assert_eq!(unpacked_level, level as u8);
    assert_eq!(unpacked_thread_id, thread_id);
  }

  #[test]
  fn test_log_event_accessors() {
    let mut event = LogEvent::new();
    let timestamp = 9876543210;
    let level = LogLevel::WARN;
    let thread_id = 123;

    event.packed_meta = LogEvent::pack_meta(timestamp, level, thread_id);

    assert_eq!(event.timestamps(), timestamp);
    assert_eq!(event.timestamp_millis(), timestamp);
    assert_eq!(event.level(), level);
    assert_eq!(event.thread_id(), thread_id);
  }

  #[test]
  fn test_log_event_reset() {
    let mut event = LogEvent::new();
    event.packed_meta = 12345;
    event.target_id = 1;
    event.message_id = NonZeroU16::new(2);
    event.kv_id = NonZeroU16::new(3);
    event.file_id = 4;
    event.position = (10, 20);

    event.reset();

    assert_eq!(event.packed_meta, 0);
    assert_eq!(event.target_id, 0);
    assert!(event.message_id.is_none());
    assert!(event.kv_id.is_none());
    assert_eq!(event.file_id, 0);
    assert_eq!(event.position, (0, 0));
  }

  #[test]
  fn test_field_value_serialization() {
    let field_values = vec![
      FieldValue::Bool(true),
      FieldValue::Bool(false),
      FieldValue::U8(255),
      FieldValue::U16(65535),
      FieldValue::U32(4294967295),
      FieldValue::U64(18446744073709551615),
      FieldValue::I8(-128),
      FieldValue::I16(-32768),
      FieldValue::I32(-2147483648),
      FieldValue::I64(-9223372036854775808),
      FieldValue::F32(3.14159),
      FieldValue::F64(2.718281828459045),
      FieldValue::StringId(42),
    ];

    for value in field_values {
      let serialized = serde_json::to_string(&value).expect("Failed to serialize");
      let deserialized: FieldValue =
        serde_json::from_str(&serialized).expect("Failed to deserialize");
      assert_eq!(deserialized, value);
    }
  }

  #[test]
  fn test_event_builder_basic() {
    let interner = Arc::new(StringInterner::new());
    let builder = EventBuilder::new(interner.clone());

    let event = builder.build_fast(12345, LogLevel::DEBUG, "test_module", "Hello World");

    assert_eq!(event.timestamp_millis(), 12345);
    assert_eq!(event.level(), LogLevel::DEBUG);
    assert!(event.kv_id.is_none());

    let target = interner.get_target(event.target_id).unwrap();
    let message = interner
      .get_message(event.message_id.unwrap().get())
      .unwrap();
    assert_eq!(target.as_ref(), "test_module");
    assert_eq!(message.as_ref(), "Hello World");
  }

  #[test]
  fn test_event_builder_with_fields() {
    let interner = Arc::new(StringInterner::new());
    let builder = EventBuilder::new(interner.clone());

    let fields = vec![
      ("key1".to_string(), FieldValue::I64(42)),
      ("key2".to_string(), FieldValue::Bool(true)),
      ("key3".to_string(), FieldValue::F64(3.14)),
    ];

    let event = builder.build_with_fields(
      67890,
      LogLevel::ERROR,
      "error_module",
      "Error occurred",
      &fields,
    );

    assert_eq!(event.timestamp_millis(), 67890);
    assert_eq!(event.level(), LogLevel::ERROR);
    assert!(event.kv_id.is_some());

    let kv_bytes = interner
      .get_kv(event.kv_id.unwrap().get())
      .expect("kv stored");
    let kv_str = std::str::from_utf8(&kv_bytes).expect("utf8");
    let parsed: serde_json::Value = serde_json::from_str(kv_str).expect("json");

    assert_eq!(parsed["key1"], serde_json::json!(42));
    assert_eq!(parsed["key2"], serde_json::json!(true));
    assert!((parsed["key3"].as_f64().unwrap() - 3.14).abs() < f64::EPSILON);
  }

  #[test]
  fn test_field_empty() {
    let field = Field::empty();
    assert_eq!(field.key_id, 0);
    assert!(matches!(field.value, FieldValue::Bool(false)));
  }

  #[test]
  fn test_log_event_display() {
    let mut event = LogEvent::new();
    event.target_id = 5;
    event.message_id = NonZeroU16::new(10);

    let display_str = format!("{}", event);
    assert_eq!(display_str, "Event(target_id=5, message_id=10)");
  }
}
