#[cfg(test)]
mod tests {
  use crate::event::Event;

  const JSON: &str = r#"{
        "ts": 1755082651423, 
        "level": 1,
        "message": "This is a test for event"
    }"#;

  #[test]
  fn test_event_serialization() {
    let ts = 1755082651423; // fixed for reproducibility

    let event = Event::new(ts, 1, String::from("This is a test for event")).serialize();
    let expected = r#"{"ts":1755082651423,"level":1,"message":"This is a test for event"}"#;

    assert_eq!(event, expected);
  }

  #[test]
  fn test_event_deserialization() {
    let event: Event = Event::deserialize(JSON.to_string());

    assert_eq!(event.ts, 1755082651423);
    assert_eq!(event.level, 1);
    assert_eq!(event.message, "This is a test for event");
  }
}
