use std::sync::atomic::Ordering;
use std::sync::Arc;

use crossbeam_channel::{bounded, unbounded};
use ttlog::event::{LogEvent, LogLevel};
use ttlog::lf_buffer::LockFreeRingBuffer;
use ttlog::string_interner::StringInterner;
use ttlog::trace::{EventBroadcast, ListenerMessage, Message, Trace, GLOBAL_LOGGER};
use ttlog::ttlog_macros::trace;

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
    Arc::new(LockFreeRingBuffer::<LogEvent>::new(128)),
  );

  let _ = GLOBAL_LOGGER.set(trace);
  GLOBAL_LOGGER
    .get()
    .expect("GLOBAL_LOGGER should be initialized for macro test")
}

#[test]
fn trace_macro_captures_changing_kv_values() {
  let logger = ensure_global_logger();
  logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);

  // Clear any prior events for deterministic assertions.
  logger.snapshot_buffer.take_snapshot();

  for i in 0..3 {
    trace!(i = i, "trace_macro_captures_changing_kv_values");
  }

  let mut observed = Vec::new();

  for event in logger.snapshot_buffer.take_snapshot() {
    let Some(message_id) = event.message_id else {
      continue;
    };

    let Some(message) = logger.interner.get_message(message_id.get()) else {
      continue;
    };

    if message.as_ref() != "trace_macro_captures_changing_kv_values" {
      continue;
    }

    let kv_id = event.kv_id.expect("expected kv_id for structured log");
    let kv_bytes = logger
      .interner
      .get_kv(kv_id.get())
      .expect("expected kv bytes in interner");
    let kv_json: serde_json::Value =
      serde_json::from_slice(&kv_bytes).expect("kv bytes should be valid JSON");

    let i_value = kv_json
      .get("i")
      .and_then(|v| v.as_i64())
      .expect("expected integer field `i`");

    observed.push(i_value);
  }

  assert_eq!(
    observed,
    vec![0, 1, 2],
    "KV values should track each invocation, got: {:?}",
    observed
  );
}
