#[cfg(test)]
mod tests {
  use crate::event::{Event, Level};

  use std::collections::HashMap;

  #[test]
  fn test_event_serialization() {
    let ts = 1755082651423;
    let mut fields = HashMap::new();
    fields.insert("key".to_string(), serde_json::json!("value"));

    let event = Event::new(
      ts,
      Level::Info,
      "This is a test for event".to_string(),
      "my_target".to_string(),
      Some(42),
      fields.clone(),
      Some(1),
      Some("my_service".to_string()),
    );

    let serialized = event.serialize();

    let expected_json = serde_json::json!({
        "timestamp": ts,
        "level": "Info",
        "message": "This is a test for event",
        "target": "my_target",
        "span_id": 42,
        "fields": fields,
        "thread_id": 1,
        "service_name": "my_service"
    })
    .to_string();

    assert_eq!(serialized, expected_json);
  }

  #[test]
  fn test_event_deserialization() {
    let json = r#"{
            "timestamp": 1755082651423,
            "level": "Info",
            "message": "This is a test for event",
            "target": "my_target",
            "span_id": 42,
            "fields": {"key": "value"},
            "thread_id": 1,
            "service_name": "my_service"
        }"#;

    let event: Event = Event::deserialize(json.to_string());

    assert_eq!(event.timestamp, 1755082651423);
    assert_eq!(event.message, "This is a test for event");
    assert_eq!(event.target, "my_target");
    assert_eq!(event.span_id, Some(42));
    assert_eq!(event.fields.get("key").unwrap(), "value");
    assert_eq!(event.thread_id, Some(1));
    assert_eq!(event.service_name.as_deref(), Some("my_service"));
  }
}
