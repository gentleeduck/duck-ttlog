#[cfg(test)]
mod __test__ {
  use std::sync::Arc;
  use crossbeam_channel::bounded;

  use tracing::info;
  use tracing_subscriber::layer::SubscriberExt;

  use crate::event::{FieldValue, LogLevel};
  use crate::string_interner::StringInterner;
  use crate::trace::Message;
  use crate::trace_layer::BufferLayer;

  fn setup_layer(cap: usize) -> (
    Arc<StringInterner>,
    crossbeam_channel::Receiver<Message>,
    tracing::subscriber::DefaultGuard,
  ) {
    let interner = Arc::new(StringInterner::new());
    let (tx, rx) = bounded::<Message>(cap);
    let layer = BufferLayer::new(tx, Arc::clone(&interner));
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let guard = tracing::subscriber::set_default(subscriber);
    (interner, rx, guard)
  }

  #[test]
  fn captures_simple_event_with_message_and_target() {
    let (interner, rx, _guard) = setup_layer(10);

    info!(target: "my_target", "hello world");

    let msg = rx.recv().expect("event received");
    match msg {
      Message::Event(ev) => {
        assert_eq!(ev.level(), LogLevel::INFO);
        // resolve interned strings
        let target = interner.get_target(ev.target_id).expect("target present");
        let message = interner.get_message(ev.message_id).expect("message present");
        assert_eq!(target.as_ref(), "my_target");
        assert_eq!(message.as_ref(), "hello world");
        assert_eq!(ev.field_count, 0);
      },
      other => panic!("unexpected message: {}", other),
    }
  }

  #[test]
  fn captures_event_with_three_fields() {
    let (interner, rx, _guard) = setup_layer(10);

    // Ensure exactly three fields (LogEvent stores up to 3)
    info!(target: "t_mod", key_i64 = -7i64, key_bool = true, key_str = "alpha", message = "payload");

    let msg = rx.recv().expect("event received");
    match msg {
      Message::Event(ev) => {
        assert_eq!(ev.level(), LogLevel::INFO);
        assert_eq!(ev.field_count, 3);

        // Verify field keys are interned and values encoded
        let k1 = interner.get_field(ev.fields[0].key_id).unwrap();
        let k2 = interner.get_field(ev.fields[1].key_id).unwrap();
        let k3 = interner.get_field(ev.fields[2].key_id).unwrap();
        let keys = [k1.as_ref(), k2.as_ref(), k3.as_ref()];

        // Collect values for flexible field order
        let mut saw_i64 = false;
        let mut saw_bool = false;
        let mut saw_str = false;

        for (i, key) in keys.iter().enumerate() {
          match (*key, &ev.fields[i].value) {
            ("key_i64", FieldValue::I64(-7)) => saw_i64 = true,
            ("key_bool", FieldValue::Bool(true)) => saw_bool = true,
            ("key_str", FieldValue::StringId(id)) => {
              let s = interner.get_field(*id).expect("field string present");
              assert_eq!(s.as_ref(), "alpha");
              saw_str = true;
            },
            (k, v) => panic!("unexpected field {k} => {:?}", v),
          }
        }

        assert!(saw_i64 && saw_bool && saw_str);

        // Resolve target/message
        let target = interner.get_target(ev.target_id).unwrap();
        let message = interner.get_message(ev.message_id).unwrap();
        assert_eq!(target.as_ref(), "t_mod");
        assert_eq!(message.as_ref(), "payload");
      },
      other => panic!("unexpected message: {}", other),
    }
  }

  #[test]
  fn channel_backpressure_is_handled_gracefully() {
    let (_interner, _rx, _guard) = setup_layer(0); // bounded(0) => always full

    // Should not panic even if channel is full
    info!(target: "t", "drop me");

    // No assertions; success is the lack of panic/crash
  }
}
