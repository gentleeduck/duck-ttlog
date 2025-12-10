#[cfg(test)]
mod __test__ {
  use std::sync::Arc;

  use crossbeam_channel::{bounded, unbounded};

  use crate::event::{LogEvent, LogLevel};
  use crate::lf_buffer::LockFreeRingBuffer;
  use crate::string_interner::StringInterner;
  use crate::trace::{EventBroadcast, ListenerMessage, Message, Trace};

  fn build_trace() -> Trace {
    let (msg_tx, _msg_rx) = bounded::<Message>(10);
    let (listener_tx, _listener_rx) = bounded::<ListenerMessage>(10);
    let (event_tx, _event_rx) = unbounded::<EventBroadcast>();
    let interner = Arc::new(StringInterner::new());
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(16));
    Trace::new(msg_tx, listener_tx, event_tx, interner, buffer)
  }

  #[test]
  fn trace_default_level_is_warn() {
    let trace = build_trace();
    assert_eq!(trace.get_level(), LogLevel::WARN);
  }

  #[test]
  fn trace_set_and_get_level() {
    let trace = build_trace();
    trace.set_level(LogLevel::ERROR);
    assert_eq!(trace.get_level(), LogLevel::ERROR);
  }

  #[test]
  fn trace_get_sender_clones_channel() {
    let (tx, rx) = bounded::<Message>(10);
    let (listener_tx, _listener_rx) = bounded::<ListenerMessage>(10);
    let (event_tx, _event_rx) = unbounded::<EventBroadcast>();
    let interner = Arc::new(StringInterner::new());
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(8));
    let trace = Trace::new(tx, listener_tx, event_tx, interner, buffer);

    let cloned = trace.get_sender();
    cloned.try_send(Message::FlushAndExit).expect("send ok");
    match rx.recv().expect("recv ok") {
      Message::FlushAndExit => {},
      _ => panic!("unexpected message variant"),
    }
  }

  #[test]
  fn message_display_formats() {
    let (tx, _rx) = std::sync::mpsc::channel();
    let d1 = format!("{}", Message::SnapshotImmediate("why".to_string(), tx));
    assert_eq!(d1, "SnapshotImmediate: why");

    let d2 = format!("{}", Message::FlushAndExit);
    assert_eq!(d2, "FlushAndExit");
  }
}
