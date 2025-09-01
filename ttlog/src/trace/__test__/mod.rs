#[cfg(test)]
mod __test__ {
  use crossbeam_channel::bounded;

  use crate::event::LogLevel;
  use crate::trace::{Message, Trace};

  #[test]
  fn trace_default_level_is_info() {
    let (tx, _rx) = bounded::<Message>(10);
    let trace = Trace::new(tx);
    assert_eq!(trace.get_level(), LogLevel::INFO);
  }

  #[test]
  fn trace_set_and_get_level() {
    let (tx, _rx) = bounded::<Message>(10);
    let trace = Trace::new(tx);

    trace.set_level(LogLevel::ERROR);
    assert_eq!(trace.get_level(), LogLevel::ERROR);

    trace.set_level(LogLevel::DEBUG);
    assert_eq!(trace.get_level(), LogLevel::DEBUG);
  }

  #[test]
  fn trace_get_sender_clones_channel() {
    let (tx, rx) = bounded::<Message>(10);
    let trace = Trace::new(tx);

    let cloned = trace.get_sender();
    cloned.try_send(Message::FlushAndExit).expect("send ok");
    let msg = rx.recv().expect("recv ok");
    match msg {
      Message::FlushAndExit => {},
      _ => panic!("unexpected message variant"),
    }
  }

  #[test]
  fn trace_request_snapshot_enqueues_message() {
    let (tx, rx) = bounded::<Message>(10);
    let trace = Trace::new(tx);

    trace.request_snapshot("manual-test");
    let msg = rx.recv().expect("recv ok");
    match msg {
      Message::SnapshotImmediate { field1: reason } => assert_eq!(reason, "manual-test"),
      _ => panic!("expected SnapshotImmediate"),
    }
  }

  #[test]
  fn message_display_formats() {
    use crate::event::LogEvent;

    let ev = LogEvent::new();
    let d1 = format!("{}", Message::Event(ev));
    assert!(d1.starts_with("Event: Event("));

    let d2 = format!(
      "{}",
      Message::SnapshotImmediate {
        field1: "why".to_string()
      }
    );
    assert_eq!(d2, "SnapshotImmediate: why");

    let d3 = format!("{}", Message::FlushAndExit);
    assert_eq!(d3, "FlushAndExit");
  }
}
