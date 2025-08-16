#[cfg(test)]
mod __test__ {

  use crate::event::LogEvent;

  #[test]
  fn test_event_creation() {
    let event = LogEvent::new(
      1234567890,
      "INFO".to_string(),
      "Test message".to_string(),
      "test_target".to_string(),
    );

    assert_eq!(event.timestamp, 1234567890);
    assert_eq!(event.level, "INFO");
    assert_eq!(event.message, "Test message");
    assert_eq!(event.target, "test_target");
  }

  #[test]
  fn test_event_default() {
    let event = LogEvent::default();

    assert_eq!(event.timestamp, 0);
    assert_eq!(event.level, "");
    assert_eq!(event.message, "");
    assert_eq!(event.target, "");
  }

  #[test]
  fn test_event_clone() {
    let event = LogEvent::new(
      1234567890,
      "WARN".to_string(),
      "Warning message".to_string(),
      "warn_target".to_string(),
    );

    let cloned = event.clone();

    assert_eq!(event.timestamp, cloned.timestamp);
    assert_eq!(event.level, cloned.level);
    assert_eq!(event.message, cloned.message);
    assert_eq!(event.target, cloned.target);
  }

  #[test]
  fn test_event_serialization() {
    let event = LogEvent::new(
      1234567890,
      "INFO".to_string(),
      "Test message".to_string(),
      "test_target".to_string(),
    );

    let json = event.serialize();
    assert!(json.contains("1234567890"));
    assert!(json.contains("INFO"));
    assert!(json.contains("Test message"));
    assert!(json.contains("test_target"));
  }

  #[test]
  fn test_event_deserialization() {
    let original_event = LogEvent::new(
      1234567890,
      "INFO".to_string(),
      "Test message".to_string(),
      "test_target".to_string(),
    );

    let json = original_event.serialize();
    let deserialized_event = LogEvent::deserialize(json);

    assert_eq!(original_event.timestamp, deserialized_event.timestamp);
    assert_eq!(original_event.level, deserialized_event.level);
    assert_eq!(original_event.message, deserialized_event.message);
    assert_eq!(original_event.target, deserialized_event.target);
  }

  #[test]
  fn test_event_display() {
    let event = LogEvent::new(
      1234567890,
      "INFO".to_string(),
      "Display test".to_string(),
      "display_target".to_string(),
    );

    let display_str = format!("{}", event);
    assert!(display_str.contains("1234567890"));
    assert!(display_str.contains("INFO"));
    assert!(display_str.contains("Display test"));
    assert!(display_str.contains("display_target"));
  }

  /// Helper to create events with different levels
  fn event_with_level(level: &str) -> LogEvent {
    LogEvent::new(
      1000,
      level.to_string(),
      "Level test".to_string(),
      "target".to_string(),
    )
  }

  #[test]
  fn test_event_all_levels() {
    let levels = ["Trace", "Debug", "Info", "Warn", "Error"];

    for &level in &levels {
      let event = event_with_level(level);
      assert_eq!(event.level, level);
      assert_eq!(event.message, "Level test");
      assert_eq!(event.target, "target");
    }
  }

  #[test]
  fn test_event_special_characters() {
    let msg = "Message with \"quotes\", newlines\n, and \\backslashes\\";
    let target = "target/with/special\\chars";
    let event = LogEvent::new(
      123,
      "DEBUG".to_string(),
      msg.to_string(),
      target.to_string(),
    );

    let json = event.serialize();
    assert!(json.contains("\\\"quotes\\\""));
    assert!(json.contains("newlines\\n"));
    assert!(json.contains("\\\\backslashes\\\\"));

    let deserialized = LogEvent::deserialize(json);
    assert_eq!(deserialized.message, msg);
    assert_eq!(deserialized.target, target);
  }

  #[test]
  fn test_event_multiple_clone_and_modify() {
    let event = LogEvent::new(
      1,
      "INFO".to_string(),
      "Original".to_string(),
      "target1".to_string(),
    );
    let mut clone1 = event.clone();
    let mut clone2 = clone1.clone();

    // Modify clones
    clone1.message = "Modified1".to_string();
    clone2.message = "Modified2".to_string();

    assert_eq!(event.message, "Original");
    assert_eq!(clone1.message, "Modified1");
    assert_eq!(clone2.message, "Modified2");
  }

  #[test]
  fn test_event_json_round_trip_with_special_chars() {
    let msg = "Special chars: \t\n\"\\";
    let event = LogEvent::new(
      999,
      "WARN".to_string(),
      msg.to_string(),
      "target".to_string(),
    );

    let json = event.serialize();
    let deserialized = LogEvent::deserialize(json.clone());
    let reserialized = deserialized.serialize();

    assert_eq!(json, reserialized);
    assert_eq!(deserialized.message, msg);
  }

  #[test]
  fn test_event_display_matches_serialize() {
    let event = LogEvent::new(
      555,
      "ERROR".to_string(),
      "Display test".to_string(),
      "display_target".to_string(),
    );
    assert_eq!(event.serialize(), format!("{}", event));
  }
}
