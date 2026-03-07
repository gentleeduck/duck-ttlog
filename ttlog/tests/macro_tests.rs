use std::sync::atomic::Ordering;
use std::sync::{Arc, Mutex};

use crossbeam_channel::{bounded, unbounded};
use ttlog::event::{LogEvent, LogLevel};
use ttlog::lf_buffer::LockFreeRingBuffer;
use ttlog::string_interner::StringInterner;
use ttlog::trace::{EventBroadcast, ListenerMessage, Message, Trace, GLOBAL_LOGGER};
use ttlog::ttlog_macros::{debug, error, fatal, info, trace, warn};

// Integration tests share GLOBAL_LOGGER so they must run serially.
// Use a helper that recovers from poison to avoid cascading failures.
static TEST_LOCK: Mutex<()> = Mutex::new(());

fn lock_tests() -> std::sync::MutexGuard<'static, ()> {
  TEST_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn ensure_global_logger() -> &'static Trace {
  if let Some(logger) = GLOBAL_LOGGER.get() {
    return logger;
  }

  let (msg_tx, _msg_rx) = bounded::<Message>(8);
  let (listener_tx, _listener_rx) = bounded::<ListenerMessage>(8);
  let (event_tx, _event_rx) = unbounded::<EventBroadcast>();

  let trace = Trace::new(
    msg_tx,
    listener_tx,
    event_tx,
    Arc::new(StringInterner::new()),
    Arc::new(LockFreeRingBuffer::<LogEvent>::new(1024)),
  );

  let _ = GLOBAL_LOGGER.set(trace);
  GLOBAL_LOGGER.get().unwrap()
}

/// Drain all events from the buffer and return only those matching the given message.
fn collect_events_with_message(logger: &Trace, expected_msg: &str) -> Vec<LogEvent> {
  logger
    .snapshot_buffer
    .take_snapshot()
    .into_iter()
    .filter(|event| {
      event
        .message_id
        .and_then(|id| logger.interner.get_message(id.get()))
        .map(|m| m.as_ref() == expected_msg)
        .unwrap_or(false)
    })
    .collect()
}

// ── All six macro levels compile and emit events ─────────────────

#[test]
fn all_macro_levels_message_only() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot(); // clear

  trace!("level_trace");
  debug!("level_debug");
  info!("level_info");
  warn!("level_warn");
  error!("level_error");
  fatal!("level_fatal");

  let events = logger.snapshot_buffer.take_snapshot();

  assert!(
    events.len() >= 6,
    "expected at least 6 events, got {}",
    events.len()
  );

  let expected_levels = [
    LogLevel::TRACE,
    LogLevel::DEBUG,
    LogLevel::INFO,
    LogLevel::WARN,
    LogLevel::ERROR,
    LogLevel::FATAL,
  ];

  let msgs = [
    "level_trace",
    "level_debug",
    "level_info",
    "level_warn",
    "level_error",
    "level_fatal",
  ];

  for (msg, expected_level) in msgs.iter().zip(expected_levels.iter()) {
    let found = events.iter().any(|e| {
      let matches_msg = e
        .message_id
        .and_then(|id| logger.interner.get_message(id.get()))
        .map(|m| m.as_ref() == *msg)
        .unwrap_or(false);
      matches_msg && e.level() == *expected_level
    });
    assert!(
      found,
      "missing event for message '{}' at level {:?}",
      msg, expected_level
    );
  }
}

// ── Empty macro call ─────────────────────────────────────────────

#[test]
fn macro_empty_call() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  trace!();

  let events = logger.snapshot_buffer.take_snapshot();
  assert!(!events.is_empty(), "empty trace!() should still emit an event");

  let event = &events[0];
  assert!(event.message_id.is_none());
  assert!(event.kv_id.is_none());
  assert_eq!(event.level(), LogLevel::TRACE);
}

// ── KV only (no message) ────────────────────────────────────────

#[test]
fn macro_kv_only() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let val = 42;
  info!(answer = val);

  let events = logger.snapshot_buffer.take_snapshot();
  assert!(!events.is_empty());

  let event = &events[0];
  assert!(
    event.message_id.is_none(),
    "kv-only call should have no message"
  );
  assert!(event.kv_id.is_some(), "kv-only call should have kv_id");

  let kv_bytes = logger
    .interner
    .get_kv(event.kv_id.unwrap().get())
    .unwrap();
  let kv: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
  // i32 uses fallback (plain number), not the string-wrapped path
  assert_eq!(kv["answer"], serde_json::json!(42));
}

// ── Message + KV ─────────────────────────────────────────────────

#[test]
fn macro_message_with_kv() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let user = "alice";
  let count = 3u64;
  warn!("user_action", user = user, count = count);

  let events = collect_events_with_message(logger, "user_action");
  assert_eq!(events.len(), 1);

  let event = &events[0];
  assert_eq!(event.level(), LogLevel::WARN);
  assert!(event.kv_id.is_some());

  let kv_bytes = logger
    .interner
    .get_kv(event.kv_id.unwrap().get())
    .unwrap();
  let kv: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
  assert_eq!(kv["user"], serde_json::json!("alice"));
  assert_eq!(kv["count"], serde_json::json!("3"));
}

// ── Multiple KV pairs ───────────────────────────────────────────

#[test]
fn macro_multiple_kv_pairs() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let a = 1i64;
  let b = 2.5f64;
  let c = true;
  debug!("multi_kv", a = a, b = b, c = c);

  let events = collect_events_with_message(logger, "multi_kv");
  assert_eq!(events.len(), 1);

  let kv_bytes = logger
    .interner
    .get_kv(events[0].kv_id.unwrap().get())
    .unwrap();
  let kv: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
  assert_eq!(kv["a"], serde_json::json!("1"));
  assert!(kv.get("b").is_some());
  assert_eq!(kv["c"], serde_json::json!(true));
}

// ── Level filtering ─────────────────────────────────────────────

#[test]
fn macro_respects_level_filter() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::ERROR as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  trace!("filtered_trace");
  debug!("filtered_debug");
  info!("filtered_info");
  warn!("filtered_warn");
  error!("filtered_error");
  fatal!("filtered_fatal");

  let events = logger.snapshot_buffer.take_snapshot();

  // Only ERROR and FATAL should pass the filter
  for event in &events {
    let level = event.level();
    assert!(
      level >= LogLevel::ERROR,
      "event with level {:?} should have been filtered",
      level
    );
  }

  let error_events = events
    .iter()
    .filter(|e| {
      e.message_id
        .and_then(|id| logger.interner.get_message(id.get()))
        .map(|m| m.as_ref() == "filtered_error")
        .unwrap_or(false)
    })
    .count();
  assert_eq!(error_events, 1);

  let fatal_events = events
    .iter()
    .filter(|e| {
      e.message_id
        .and_then(|id| logger.interner.get_message(id.get()))
        .map(|m| m.as_ref() == "filtered_fatal")
        .unwrap_or(false)
    })
    .count();
  assert_eq!(fatal_events, 1);
}

// ── Expression values in KV ─────────────────────────────────────

#[test]
fn macro_kv_with_expressions() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let x = 10i64;
  let doubled = x * 2;
  let sum = x + 5;
  trace!("expr_kv", doubled = doubled, sum = sum);

  let events = collect_events_with_message(logger, "expr_kv");
  assert_eq!(events.len(), 1);

  let kv_bytes = logger
    .interner
    .get_kv(events[0].kv_id.unwrap().get())
    .unwrap();
  let kv: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
  assert_eq!(kv["doubled"], serde_json::json!("20"));
  assert_eq!(kv["sum"], serde_json::json!("15"));
}

// ── Target and file are interned ────────────────────────────────

#[test]
fn macro_interns_target_and_file() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  info!("interned_check");

  let events = collect_events_with_message(logger, "interned_check");
  assert_eq!(events.len(), 1);

  let event = &events[0];
  let target = logger.interner.get_target(event.target_id);
  assert!(target.is_some(), "target should be interned");

  let file = logger.interner.get_file(event.file_id);
  assert!(file.is_some(), "file should be interned");
  assert!(
    file.unwrap().contains("macro_tests"),
    "file should reference this test file"
  );
}

// ── Position (line, column) is non-zero ─────────────────────────

#[test]
fn macro_captures_position() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  error!("position_check");

  let events = collect_events_with_message(logger, "position_check");
  assert_eq!(events.len(), 1);

  let (line, col) = events[0].position;
  assert!(line > 0, "line should be non-zero");
  assert!(col > 0, "column should be non-zero");
}

// ── Timestamp is reasonable ─────────────────────────────────────

#[test]
fn macro_timestamp_is_recent() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let before = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

  info!("timestamp_check");

  let after = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap()
    .as_millis() as u64;

  let events = collect_events_with_message(logger, "timestamp_check");
  assert_eq!(events.len(), 1);

  let ts = events[0].timestamp_millis();
  assert!(ts >= before, "timestamp {} should be >= before {}", ts, before);
  assert!(ts <= after, "timestamp {} should be <= after {}", ts, after);
}

// ── String KV values ────────────────────────────────────────────

#[test]
fn macro_string_kv_values() {
  let _lock = lock_tests();
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);
  logger.snapshot_buffer.take_snapshot();

  let name = "bob";
  let msg = "hello world";
  info!("string_kv", name = name, msg = msg);

  let events = collect_events_with_message(logger, "string_kv");
  assert_eq!(events.len(), 1);

  let kv_bytes = logger
    .interner
    .get_kv(events[0].kv_id.unwrap().get())
    .unwrap();
  let kv: serde_json::Value = serde_json::from_slice(&kv_bytes).unwrap();
  assert_eq!(kv["name"], serde_json::json!("bob"));
  assert_eq!(kv["msg"], serde_json::json!("hello world"));
}
