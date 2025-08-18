#[cfg(test)]
mod __test__ {

  // tests/log_event_tests.rs

  use std::borrow::Cow;

  use serde_json;

  use crate::event::{EventBuilder, FieldValue, LogLevel};

  #[test]
  fn test_event_builder_basic() {
    let mut builder = EventBuilder::new_with_capacity(4);
    builder
      .timestamp_nanos(12345)
      .level(LogLevel::DEBUG)
      .target(Cow::Borrowed("test_module"))
      .message(Cow::Borrowed("Hello World"))
      .field("key1", FieldValue::I64(42))
      .field("key2", FieldValue::Str(Cow::Borrowed("static_str")));

    let event = builder.build();

    assert_eq!(event.timestamp_nanos, 12345);
    assert_eq!(event.level, LogLevel::DEBUG);
    assert_eq!(event.target, "test_module");
    assert_eq!(event.message, "Hello World");
    assert_eq!(event.fields.len(), 2);

    assert_eq!(event.fields[0].key, "key1");
    assert!(matches!(event.fields[0].value, FieldValue::I64(42)));
    assert_eq!(event.fields[1].key, "key2");
    assert!(matches!(
      event.fields[1].value,
      FieldValue::Str(Cow::Borrowed("static_str"))
    ));
  }

  #[test]
  fn test_field_value_serialization() {
    let field_values = vec![
      FieldValue::Str(Cow::Borrowed("static")),
      FieldValue::String("owned".to_string()),
      FieldValue::I64(-123),
      FieldValue::U64(456),
      FieldValue::F64(3.14),
      FieldValue::Bool(true),
      FieldValue::Debug("dbg".to_string()),
      FieldValue::Display("disp".to_string()),
    ];

    for value in field_values {
      let serialized = serde_json::to_string(&value).expect("Failed to serialize");
      let deserialized: FieldValue =
        serde_json::from_str(&serialized).expect("Failed to deserialize");

      // Str(Cow<'static, str>),
      // String(String),
      // Debug(String),
      // Display(String),
      // Null,
      // None,
      // Bool(bool),
      // U8(u8),
      // U16(u16),
      // U32(u32),
      // U64(u64),
      // I8(i8),
      // I16(i16),
      // I32(i32),
      // I64(i64),
      // F32(f32),
      // F64(f64),
      match value {
        FieldValue::Str(s) => {
          if let FieldValue::Str(ds) = deserialized {
            assert_eq!(ds, s);
          } else {
            panic!("Expected Str variant");
          }
        },
        FieldValue::String(ref s) => {
          if let FieldValue::String(ds) = deserialized {
            assert_eq!(ds, *s);
          } else {
            panic!("Expected String variant");
          }
        },
        FieldValue::I64(i) => {
          if let FieldValue::I64(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I64 variant");
          }
        },
        FieldValue::U64(u) => {
          if let FieldValue::U64(d) = deserialized {
            assert_eq!(d, u);
          } else {
            panic!("Expected U64 variant");
          }
        },
        FieldValue::F64(f) => {
          if let FieldValue::F64(d) = deserialized {
            assert!((d - f).abs() < f64::EPSILON);
          } else {
            panic!("Expected F64 variant");
          }
        },
        FieldValue::I8(i) => {
          if let FieldValue::I8(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected I8 variant");
          }
        },
        FieldValue::Bool(b) => {
          if let FieldValue::Bool(d) = deserialized {
            assert_eq!(d, b);
          } else {
            panic!("Expected Bool variant");
          }
        },
        FieldValue::Debug(ref s) => {
          if let FieldValue::Debug(ds) = deserialized {
            assert_eq!(ds, *s);
          } else {
            panic!("Expected Debug variant");
          }
        },
        FieldValue::Display(ref s) => {
          if let FieldValue::Display(ds) = deserialized {
            assert_eq!(ds, *s);
          } else {
            panic!("Expected Display variant");
          }
        },
        FieldValue::None => {
          if let FieldValue::None = deserialized {
            assert!(true);
          } else {
            panic!("Expected None variant");
          }
        },
        FieldValue::Null => {
          if let FieldValue::Null = deserialized {
            assert!(true);
          } else {
            panic!("Expected Null variant");
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
        FieldValue::F32(i) => {
          if let FieldValue::F32(d) = deserialized {
            assert_eq!(d, i);
          } else {
            panic!("Expected F32 variant");
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
      }
    }
  }

  #[test]
  fn test_event_builder_clone() {
    let mut builder = EventBuilder::new_with_capacity(2);
    builder
      .message("Clone test".into())
      .field("a", FieldValue::I64(1));

    let event1 = builder.build();
    let event2 = event1.clone();

    assert_eq!(event1.message, event2.message);
    assert_eq!(event1.fields.len(), event2.fields.len());
  }

  #[test]
  fn test_event_builder_multiple_fields() {
    let keys = [
      "key0", "key1", "key2", "key3", "key4", "key5", "key6", "key7",
    ];

    let mut builder = EventBuilder::new_with_capacity(8);
    for i in 0..8 {
      builder.field(keys[i], FieldValue::I64(i as i64));
    }

    let event = builder.build();
    assert_eq!(event.fields.len(), 8);

    for (i, field) in event.fields.iter().enumerate() {
      assert_eq!(field.key, keys[i]);
      assert!(matches!(field.value, FieldValue::I64(v) if v == i as i64));
    }
  }
}
