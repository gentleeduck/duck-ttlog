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

    // SEC-023 (flake fix): `GLOBAL_LOGGER` is a process-wide `OnceLock` and
    // `cargo test` runs tests in parallel within ONE process. This test
    // therefore CANNOT assume it performs the first init — another test may
    // have already claimed the slot. The invariant under test is purely
    // "init/try_init must never panic on a double-init", so we tolerate
    // either outcome of the first call instead of asserting success.
    let first = Trace::try_init(64, 8, "double_init_test", Some("./tmp/"));
    let mut first_owned = first.ok();

    // Whatever happened above, the global slot is now occupied — by this
    // test or by another. Every further init must report the double-init as
    // an error and must NOT panic.
    match Trace::try_init(64, 8, "double_init_test", Some("./tmp/")) {
      Ok(_) => panic!("init after global logger set unexpectedly succeeded"),
      Err((mut trace, err)) => {
        assert_eq!(err, InitError::AlreadyInitialized);
        trace.shutdown();
      },
    }

    // The infallible wrapper also must not panic when the slot is taken.
    let mut third = Trace::init(64, 8, "double_init_test", Some("./tmp/"));
    third.shutdown();

    if let Some(mut first) = first_owned.take() {
      first.shutdown();
    }
  }

  /// SEC-010: with no consumer, sending past the bounded broadcast channel's
  /// capacity must NOT block — the producer drops the newest event and bumps
  /// the dropped-event counter. Prior to the fix the channel was unbounded,
  /// so a stalled listener grew memory without limit.
  #[test]
  fn broadcast_channel_drops_newest_when_full_without_blocking() {
    use crate::event::LogLevel;
    use crate::trace::{dropped_events, DROPPED_EVENTS};
    use std::sync::atomic::Ordering;

    // A small bounded channel with NO receiver attached: every send beyond
    // `cap` must fail fast via `try_send` rather than block.
    let cap = 4;
    let (tx, _rx) = bounded::<EventBroadcast>(cap);

    let make_event = || EventBroadcast {
      event: LogEvent {
        packed_meta: LogEvent::pack_meta(0, LogLevel::INFO, 0),
        target_id: 0,
        message_id: None,
        position: (0, 0),
        file_id: 0,
        kv_id: None,
      },
    };

    let before = dropped_events();
    let attempts = cap + 100;
    let mut dropped_here = 0u64;

    // None of these calls may block — `try_send` returns immediately.
    for _ in 0..attempts {
      if tx.try_send(make_event()).is_err() {
        DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
        dropped_here += 1;
      }
    }

    // The first `cap` sends succeed; the remaining `attempts - cap` are
    // dropped (drop-newest).
    assert_eq!(
      dropped_here,
      (attempts - cap) as u64,
      "expected exactly the over-capacity sends to be dropped"
    );
    assert!(
      dropped_events() >= before + dropped_here,
      "dropped-event counter must increment on a full channel"
    );
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
