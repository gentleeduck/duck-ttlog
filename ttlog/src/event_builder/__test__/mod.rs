#[cfg(test)]
mod __test__ {

  use std::sync::Arc;

  use crate::event::{FieldValue, LogLevel};
  use crate::event_builder::EventBuilder;
  use crate::string_interner::StringInterner;

  fn make_builder() -> (EventBuilder, Arc<StringInterner>) {
    let interner = Arc::new(StringInterner::new());
    let builder = EventBuilder::new(interner.clone());
    (builder, interner)
  }

  #[test]
  fn build_fast_sets_timestamp_and_level() {
    let (builder, _) = make_builder();
    let event = builder.build_fast(5000, LogLevel::WARN, "mod", "msg");
    assert_eq!(event.timestamp_millis(), 5000);
    assert_eq!(event.level(), LogLevel::WARN);
  }

  #[test]
  fn build_fast_interns_target_and_message() {
    let (builder, interner) = make_builder();
    let event = builder.build_fast(0, LogLevel::INFO, "my_mod", "hello");

    let target = interner.get_target(event.target_id).unwrap();
    assert_eq!(target.as_ref(), "my_mod");

    let msg = interner
      .get_message(event.message_id.unwrap().get())
      .unwrap();
    assert_eq!(msg.as_ref(), "hello");
  }

  #[test]
  fn build_fast_has_no_kv() {
    let (builder, _) = make_builder();
    let event = builder.build_fast(0, LogLevel::TRACE, "t", "m");
    assert!(event.kv_id.is_none());
  }

  #[test]
  fn build_fast_thread_id_is_zero() {
    let (builder, _) = make_builder();
    let event = builder.build_fast(0, LogLevel::DEBUG, "t", "m");
    assert_eq!(event.thread_id(), 0);
  }

  #[test]
  fn build_with_fields_stores_kv() {
    let (builder, interner) = make_builder();
    let fields = vec![
      ("count".to_string(), FieldValue::U32(10)),
      ("active".to_string(), FieldValue::Bool(true)),
    ];

    let event = builder.build_with_fields(100, LogLevel::ERROR, "srv", "fail", &fields);
    assert!(event.kv_id.is_some());

    let kv_bytes = interner.get_kv(event.kv_id.unwrap().get()).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
    assert_eq!(parsed["count"], serde_json::json!(10));
    assert_eq!(parsed["active"], serde_json::json!(true));
  }

  #[test]
  fn build_with_empty_fields_has_no_kv() {
    let (builder, _) = make_builder();
    let event = builder.build_with_fields(0, LogLevel::INFO, "t", "m", &[]);
    assert!(event.kv_id.is_none());
  }

  #[test]
  fn build_with_fields_preserves_float() {
    let (builder, interner) = make_builder();
    let fields = vec![("pi".to_string(), FieldValue::F64(3.14159))];

    let event = builder.build_with_fields(0, LogLevel::DEBUG, "math", "const", &fields);
    let kv_bytes = interner.get_kv(event.kv_id.unwrap().get()).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
    let pi = parsed["pi"].as_f64().unwrap();
    assert!((pi - 3.14159).abs() < 1e-5);
  }

  #[test]
  fn build_with_fields_negative_integers() {
    let (builder, interner) = make_builder();
    let fields = vec![
      ("code".to_string(), FieldValue::I32(-1)),
      ("big".to_string(), FieldValue::I64(-9999999)),
    ];

    let event = builder.build_with_fields(0, LogLevel::WARN, "err", "neg", &fields);
    let kv_bytes = interner.get_kv(event.kv_id.unwrap().get()).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
    assert_eq!(parsed["code"], serde_json::json!(-1));
    assert_eq!(parsed["big"], serde_json::json!(-9999999));
  }

  #[test]
  fn build_fast_all_levels() {
    let (builder, _) = make_builder();

    let levels = [
      LogLevel::TRACE,
      LogLevel::DEBUG,
      LogLevel::INFO,
      LogLevel::WARN,
      LogLevel::ERROR,
      LogLevel::FATAL,
    ];

    for level in levels {
      let event = builder.build_fast(0, level, "t", "m");
      assert_eq!(event.level(), level);
    }
  }

  #[test]
  fn same_strings_get_same_ids() {
    let (builder, _) = make_builder();
    let e1 = builder.build_fast(0, LogLevel::INFO, "mod_a", "msg_x");
    let e2 = builder.build_fast(0, LogLevel::INFO, "mod_a", "msg_x");
    assert_eq!(e1.target_id, e2.target_id);
    assert_eq!(e1.message_id, e2.message_id);
  }

  #[test]
  fn different_strings_get_different_ids() {
    let (builder, _) = make_builder();
    let e1 = builder.build_fast(0, LogLevel::INFO, "mod_a", "msg_x");
    let e2 = builder.build_fast(0, LogLevel::INFO, "mod_b", "msg_y");
    assert_ne!(e1.target_id, e2.target_id);
    assert_ne!(e1.message_id, e2.message_id);
  }

  #[test]
  fn build_with_fields_string_id_variant() {
    let (builder, interner) = make_builder();
    let fields = vec![("ref_id".to_string(), FieldValue::StringId(42))];

    let event = builder.build_with_fields(0, LogLevel::INFO, "t", "m", &fields);
    let kv_bytes = interner.get_kv(event.kv_id.unwrap().get()).unwrap();
    let parsed: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
    assert_eq!(parsed["ref_id"], serde_json::json!(42));
  }

  #[test]
  fn build_fast_file_id_and_position_default_to_zero() {
    let (builder, _) = make_builder();
    let event = builder.build_fast(0, LogLevel::INFO, "t", "m");
    assert_eq!(event.file_id, 0);
    assert_eq!(event.position, (0, 0));
  }
}
