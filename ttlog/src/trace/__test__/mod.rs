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

  #[test]
  fn double_init_returns_err_without_panic() {
    use crate::trace::InitError;

    // First init succeeds and registers the global logger.
    let mut first = match Trace::try_init(64, 8, "double_init_test", Some("./tmp/")) {
      Ok(trace) => trace,
      Err(_) => panic!("first init should succeed"),
    };

    // Second init must NOT panic — it reports the double-init as an error.
    match Trace::try_init(64, 8, "double_init_test", Some("./tmp/")) {
      Ok(_) => panic!("second init unexpectedly succeeded"),
      Err((mut trace, err)) => {
        assert_eq!(err, InitError::AlreadyInitialized);
        trace.shutdown();
      },
    }

    // The infallible wrapper also must not panic on double-init.
    let mut third = Trace::init(64, 8, "double_init_test", Some("./tmp/"));
    third.shutdown();

    first.shutdown();
  }

  /// SEC-021: a double-init must spawn ZERO extra threads. Prior to the fix
  /// `try_init` spawned the writer + listener threads before checking the
  /// global slot, leaking two threads per redundant call.
  #[test]
  #[cfg(target_os = "linux")]
  fn double_init_spawns_no_extra_threads() {
    fn task_count() -> usize {
      std::fs::read_dir("/proc/self/task")
        .map(|d| d.count())
        .unwrap_or(0)
    }

    // First init claims the global slot (may legitimately spawn threads if
    // this test wins the race for the process-wide GLOBAL_LOGGER).
    let _ = Trace::try_init(64, 8, "sec021_thread_probe", Some("./tmp/"));

    // Let any threads spawned by the first init settle.
    std::thread::sleep(std::time::Duration::from_millis(50));
    let before = task_count();

    // Redundant inits: every one of these must hit the early-return path.
    for _ in 0..8 {
      let result = Trace::try_init(64, 8, "sec021_thread_probe", Some("./tmp/"));
      assert!(
        result.is_err(),
        "redundant try_init must report AlreadyInitialized"
      );
    }

    std::thread::sleep(std::time::Duration::from_millis(50));
    let after = task_count();

    assert!(
      after <= before,
      "double-init leaked threads: {} -> {} after 8 redundant inits",
      before,
      after
    );
  }
}
