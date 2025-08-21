#[cfg(test)]
mod __test__ {

  use serde_json;
  use std::sync::Arc;

  use crate::event::{Field, FieldValue, LogEvent, LogLevel};
  use crate::event_builder::EventBuilder;
  use crate::string_interner::StringInterner;

  #[test]
  fn test_log_level_from_str() {
    assert_eq!(LogLevel::from_str("trace"), LogLevel::TRACE);
    assert_eq!(LogLevel::from_str("debug"), LogLevel::DEBUG);
    assert_eq!(LogLevel::from_str("info"), LogLevel::INFO);
    assert_eq!(LogLevel::from_str("warn"), LogLevel::WARN);
    assert_eq!(LogLevel::from_str("error"), LogLevel::ERROR);
    assert_eq!(LogLevel::from_str("unknown"), LogLevel::INFO); // default
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
    assert_eq!(event.message_id, 0);
    assert_eq!(event.field_count, 0);
    assert_eq!(event.file_id, 0);
    assert_eq!(event.line, 0);
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

    assert_eq!(event.timestamp_millis(), timestamp);
    assert_eq!(event.level(), level);
    assert_eq!(event.thread_id(), thread_id);
  }

  #[test]
  fn test_log_event_add_field() {
    let mut event = LogEvent::new();

    // Add first field
    assert!(event.add_field(1, FieldValue::I64(42)));
    assert_eq!(event.field_count, 1);
    assert_eq!(event.fields[0].key_id, 1);
    assert!(matches!(event.fields[0].value, FieldValue::I64(42)));

    // Add second field
    assert!(event.add_field(2, FieldValue::Bool(true)));
    assert_eq!(event.field_count, 2);

    // Add third field
    assert!(event.add_field(3, FieldValue::F64(3.14)));
    assert_eq!(event.field_count, 3);

    // Try to add fourth field (should fail - max 3 fields)
    assert!(!event.add_field(4, FieldValue::U64(999)));
    assert_eq!(event.field_count, 3);
  }

  #[test]
  fn test_log_event_reset() {
    let mut event = LogEvent::new();
    event.packed_meta = 12345;
    event.target_id = 1;
    event.message_id = 2;
    event.field_count = 2;
    event.file_id = 3;
    event.line = 100;

    event.reset();

    assert_eq!(event.packed_meta, 0);
    assert_eq!(event.target_id, 0);
    assert_eq!(event.message_id, 0);
    assert_eq!(event.field_count, 0);
    assert_eq!(event.file_id, 0);
    assert_eq!(event.line, 0);
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

      match value {
        FieldValue::Bool(b) => {
          if let FieldValue::Bool(d) = deserialized {
            assert_eq!(d, b);
          } else {
            panic!("Expected Bool variant");
          }
        },
        FieldValue::U8(u) => {
          if let FieldValue::U8(d) = deserialized {
            assert_eq!(d, u);
          } else {
            panic!("Expected U8 variant");
          }
        },
        FieldValue::U16(u) => {
          if let FieldValue::U16(d) = deserialized {
            assert_eq!(d, u);
          } else {
            panic!("Expected U16 variant");
          }
        },
        FieldValue::U32(u) => {
          if let FieldValue::U32(d) = deserialized {
            assert_eq!(d, u);
          } else {
            panic!("Expected U32 variant");
          }
        },
        FieldValue::U64(u) => {
          if let FieldValue::U64(d) = deserialized {
            assert_eq!(d, u);
          } else {
            panic!("Expected U64 variant");
          }
        },
        FieldValue::I8(i) => {
          if let FieldValue::I8(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I8 variant");
          }
        },
        FieldValue::I16(i) => {
          if let FieldValue::I16(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I16 variant");
          }
        },
        FieldValue::I32(i) => {
          if let FieldValue::I32(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I32 variant");
          }
        },
        FieldValue::I64(i) => {
          if let FieldValue::I64(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I64 variant");
          }
        },
        FieldValue::F32(f) => {
          if let FieldValue::F32(d) = deserialized {
            assert!((d - f).abs() < f32::EPSILON);
          } else {
            panic!("Expected F32 variant");
          }
        },
        FieldValue::F64(f) => {
          if let FieldValue::F64(d) = deserialized {
            assert!((d - f).abs() < f64::EPSILON);
          } else {
            panic!("Expected F64 variant");
          }
        },
        FieldValue::StringId(id) => {
          if let FieldValue::StringId(d) = deserialized {
            assert_eq!(d, id);
          } else {
            panic!("Expected StringId variant");
          }
        },
      }
    }
  }

  #[test]
  fn test_event_builder_basic() {
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner.clone());

    let event = builder.build_fast(12345, LogLevel::DEBUG, "test_module", "Hello World");

    assert_eq!(event.timestamp_millis(), 12345);
    assert_eq!(event.level(), LogLevel::DEBUG);
    assert_eq!(event.field_count, 0);

    // Verify string interning worked
    let target = interner.get_target(event.target_id).unwrap();
    let message = interner.get_message(event.message_id).unwrap();
    assert_eq!(target.as_ref(), "test_module");
    assert_eq!(message.as_ref(), "Hello World");
  }

  #[test]
  fn test_event_builder_with_fields() {
    let interner = Arc::new(StringInterner::new());
    let mut builder = EventBuilder::new(interner.clone());

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
    assert_eq!(event.field_count, 3);

    // Check field values
    assert!(matches!(event.fields[0].value, FieldValue::I64(42)));
    assert!(matches!(event.fields[1].value, FieldValue::Bool(true)));
    assert!(matches!(event.fields[2].value, FieldValue::F64(f) if (f - 3.14).abs() < f64::EPSILON));

    // Verify string interning
    let target = interner.get_target(event.target_id).unwrap();
    let message = interner.get_message(event.message_id).unwrap();
    assert_eq!(target.as_ref(), "error_module");
    assert_eq!(message.as_ref(), "Error occurred");

    // Verify field keys are interned
    let key1 = interner.get_kv(event.fields[0].key_id).unwrap();
    let key2 = interner.get_kv(event.fields[1].key_id).unwrap();
    let key3 = interner.get_kv(event.fields[2].key_id).unwrap();
    assert_eq!(key1.as_ref(), "key1");
    assert_eq!(key2.as_ref(), "key2");
    assert_eq!(key3.as_ref(), "key3");
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
    event.message_id = 10;

    let display_str = format!("{}", event);
    assert_eq!(display_str, "Event(target_id=5, message_id=10)");
  }
}
