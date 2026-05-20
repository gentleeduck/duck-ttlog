mod __test__;

use chrono::Duration;
use std::num;
use std::sync::OnceLock;
use std::time::{Instant, SystemTime, UNIX_EPOCH};
use std::{sync::Arc, thread};

use crate::event::{LogEvent, LogLevel};
use crate::lf_buffer::LockFreeRingBuffer;
use crate::listener::LogListener;
use crate::panic_hook::PanicHook;
use crate::snapshot::SnapshotWriter;
use crate::string_interner::StringInterner;
use crossbeam_channel::{Receiver, Sender};
use std::sync::atomic::{self, AtomicU64, AtomicU8, Ordering};

/// SEC-010: capacity of the event-broadcast channel.
///
/// The broadcast channel is bounded so a stalled or absent listener cannot
/// cause unbounded memory growth on the hot logging path. When the channel is
/// full the producer side uses `try_send` and **drops the newest event**
/// (drop-newest policy) rather than blocking — backpressure must never reach
/// the caller of a logging macro. Dropped events are counted in
/// [`DROPPED_EVENTS`] for observability.
const EVENT_BROADCAST_CAPACITY: usize = 8192;

/// SEC-010: total number of broadcast events dropped because the bounded
/// channel was full (drop-newest policy). Monotonic, process-wide.
pub static DROPPED_EVENTS: AtomicU64 = AtomicU64::new(0);

/// Returns the number of broadcast events dropped so far due to a full
/// channel (see [`EVENT_BROADCAST_CAPACITY`]).
pub fn dropped_events() -> u64 {
  DROPPED_EVENTS.load(Ordering::Relaxed)
}

#[derive(Debug)]
pub enum Message {
  SnapshotImmediate(String, std::sync::mpsc::Sender<()>),
  FlushAndExit,
}

pub enum ListenerMessage {
  Add(
    Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
    std::sync::mpsc::Sender<()>,
  ),
  Shutdown,
}

// New message type for direct event broadcasting
#[derive(Debug, Clone)]
pub struct EventBroadcast {
  pub event: LogEvent,
}

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Message::SnapshotImmediate(reason, _) => write!(f, "SnapshotImmediate: {}", reason),
      Message::FlushAndExit => write!(f, "FlushAndExit"),
    }
  }
}

pub struct Trace {
  /// For snapshots (accumulates events, only drained on snapshot)
  pub snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  /// Channel sender for communicating with the writer thread
  pub sender: Sender<Message>,
  /// Direct event broadcasting channel — bounded (SEC-010). On a full channel
  /// the newest event is dropped (see [`EVENT_BROADCAST_CAPACITY`]).
  pub event_broadcast_sender: Sender<EventBroadcast>,
  /// Atomic log level for runtime filtering
  pub level: atomic::AtomicU8,
  pub interner: Arc<StringInterner>,
  pub listener_sender: Sender<ListenerMessage>,
  pub write_thread: Option<thread::JoinHandle<()>>,
  pub listener_thread: Option<thread::JoinHandle<()>>,
}

pub static GLOBAL_LOGGER: OnceLock<Trace> = OnceLock::new();

/// Error returned by [`Trace::try_init`] when initialization cannot complete.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum InitError {
  /// `Trace::init`/`try_init` was called more than once in this process.
  AlreadyInitialized,
}

impl std::fmt::Display for InitError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      InitError::AlreadyInitialized => write!(f, "GLOBAL_LOGGER already initialized"),
    }
  }
}

impl std::error::Error for InitError {}

impl Trace {
  pub fn new(
    sender: Sender<Message>,
    listener_sender: Sender<ListenerMessage>,
    event_broadcast_sender: Sender<EventBroadcast>,
    interner: Arc<StringInterner>,
    snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  ) -> Self {
    Self {
      sender,
      event_broadcast_sender,
      snapshot_buffer,
      interner,
      listener_sender,
      level: AtomicU8::new(LogLevel::WARN as u8),
      write_thread: None,
      listener_thread: None,
    }
  }

  fn set_handler(
    &mut self,
    write_thread: Option<thread::JoinHandle<()>>,
    listener_thread: Option<thread::JoinHandle<()>>,
  ) {
    self.write_thread = write_thread;
    self.listener_thread = listener_thread;
  }

  /// Initializes the global logger.
  ///
  /// On a double-init this logs a warning and returns the freshly built
  /// `Trace` without replacing the existing global logger, rather than
  /// panicking. Use [`Trace::try_init`] to observe the double-init as an
  /// error instead.
  pub fn init(
    capacity: usize,
    channel_capacity: usize,
    service_name: &str,
    storage_path: Option<&str>,
  ) -> Self {
    match Self::try_init(capacity, channel_capacity, service_name, storage_path) {
      Ok(trace) => trace,
      Err((trace, err)) => {
        eprintln!(
          "[Trace] {} — continuing without replacing global logger",
          err
        );
        trace
      },
    }
  }

  /// Fallible variant of [`Trace::init`].
  ///
  /// On success returns the built `Trace`. On a double-init returns
  /// `Err((trace, InitError::AlreadyInitialized))` — the `trace` is still
  /// usable locally; it just is not registered as the global logger.
  pub fn try_init(
    capacity: usize,
    channel_capacity: usize,
    service_name: &str,
    storage_path: Option<&str>,
  ) -> Result<Self, (Self, InitError)> {
    // SEC-021: gate on the global init state BEFORE spawning any thread.
    // A double-init must not leak the writer + listener threads or their
    // channels. The cheap `get()` short-circuits the common redundant-call
    // case; the authoritative claim is the `OnceLock::set` below, which
    // closes the TOCTOU between this check and the actual claim.
    if let Some(existing) = GLOBAL_LOGGER.get() {
      eprintln!("[Trace] Warning: GLOBAL_LOGGER already initialized");
      return Err((existing.clone(), InitError::AlreadyInitialized));
    }

    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let (listener_sender, listener_receiver) = crossbeam_channel::bounded::<ListenerMessage>(16);

    // SEC-010: bounded broadcast channel. A stalled listener can no longer
    // grow this queue without limit; once full, producers drop the newest
    // event via `try_send` instead of blocking the logging hot path.
    let (event_broadcast_sender, event_broadcast_receiver) =
      crossbeam_channel::bounded::<EventBroadcast>(EVENT_BROADCAST_CAPACITY);

    let interner = Arc::new(StringInterner::new());

    // Only need snapshot buffer now - listeners get events directly
    let snapshot_buffer = Arc::new(LockFreeRingBuffer::new(capacity));
    let snapshot_buffer_clone = Arc::clone(&snapshot_buffer);
    let interner_clone = Arc::clone(&interner);

    let service_name = service_name.to_string();
    let storage_path: String = match storage_path {
      Some(path) => path.to_string(),
      None => "./tmp/".to_string(),
    };

    // Create the trace instance first
    let mut trace = Trace::new(
      sender,
      listener_sender,
      event_broadcast_sender,
      interner,
      snapshot_buffer,
    );

    // Authoritatively claim the global slot. If another thread won the race
    // between the `get()` above and here, `set` returns Err — we bail out
    // WITHOUT spawning any thread or installing the panic hook.
    if GLOBAL_LOGGER.set(trace.clone()).is_err() {
      eprintln!("[Trace] Warning: GLOBAL_LOGGER already initialized");
      let existing = GLOBAL_LOGGER.get().cloned().unwrap_or(trace);
      return Err((existing, InitError::AlreadyInitialized));
    }

    println!("GLOBAL_LOGGER initialized");

    // Claim succeeded — only now do we spawn background threads and install
    // the panic hook, so a redundant call can never leak them.
    PanicHook::install(trace.sender.clone());

    let write_thread_handle = thread::spawn(move || {
      Self::writer_loop(
        receiver,
        capacity,
        service_name,
        storage_path,
        snapshot_buffer_clone,
        interner_clone,
      );
    });

    // Spawn separate listener management thread
    let interner_listener = Arc::clone(&trace.interner);
    let listener_thread_handle = thread::spawn(move || {
      Self::listener_loop(
        listener_receiver,
        event_broadcast_receiver,
        interner_listener,
      );
    });

    trace.set_handler(Some(write_thread_handle), Some(listener_thread_handle));

    Ok(trace)
  }

  pub fn add_listener(
    &self,
    listener: Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
  ) {
    let (ack_tx, ack_rx) = std::sync::mpsc::channel();
    match self
      .listener_sender
      .send(ListenerMessage::Add(listener, ack_tx))
    {
      Ok(_) => {
        println!("[Trace] Listener addition request sent");
        if ack_rx.recv().is_err() {
          eprintln!("[Trace] Listener addition ack channel dropped");
        }
      },
      Err(e) => {
        eprintln!("[Trace] Failed to add listener: {:?}", e);
      },
    }
  }

  pub fn shutdown(&mut self) {
    // Shutdown listeners first
    let _ = self.listener_sender.try_send(ListenerMessage::Shutdown);
    // Then shutdown writer
    let _ = self.sender.try_send(Message::FlushAndExit);

    // join threads instead of sleeping
    if let Some(handle) = self.write_thread.take() {
      let _ = handle.join();
    }
    if let Some(handle) = self.listener_thread.take() {
      let _ = handle.join();
    }

    eprintln!("[Trace] Shutdown completed");
  }

  pub fn get_sender(&self) -> Sender<Message> {
    self.sender.clone()
  }

  pub fn request_snapshot(&self, reason: impl Into<String>) {
    let (tx, rx) = std::sync::mpsc::channel();

    if let Err(e) = self
      .sender
      .try_send(Message::SnapshotImmediate(reason.into(), tx))
    {
      eprintln!("[Snapshot Request] Failed to enqueue: {:?}", e);
      return;
    }

    eprintln!("[Snapshot Request] Waiting for snapshot completion...");
    if rx.recv().is_ok() {
      eprintln!("[Snapshot Request] Snapshot completed!");
    }
  }

  pub fn set_level(&self, level: LogLevel) {
    self.level.store(level as u8, Ordering::Relaxed);
    if let Some(logger) = GLOBAL_LOGGER.get() {
      logger.level.store(level as u8, Ordering::Relaxed);
    }
  }

  pub fn get_level(&self) -> LogLevel {
    let level_u8 = self.level.load(Ordering::Relaxed);
    LogLevel::from_u8(&level_u8)
  }

  #[inline(always)]
  pub fn send_event_fast(
    &self,
    log_level: u8,
    target_id: u16,
    message_id: Option<num::NonZeroU16>,
    thread_id: u8,
    file_id: u16,
    position: (u32, u32),
    kv_id: Option<num::NonZeroU16>,
  ) {
    let timestamp = SystemTime::now()
      .duration_since(UNIX_EPOCH)
      .unwrap_or_default()
      .as_millis() as u64;

    let event = LogEvent {
      packed_meta: LogEvent::pack_meta(
        timestamp,
        LogLevel::from_u8(&(log_level & 0x07)),
        thread_id,
      ),
      target_id,
      message_id,
      position,
      file_id,
      kv_id,
    };

    // Add to snapshot buffer for periodic snapshots
    self.snapshot_buffer.push_overwrite(event.clone());

    // SEC-010: broadcast over the BOUNDED channel with `try_send` — a
    // drop-newest policy. If the channel is full (a stalled/slow listener)
    // the event is dropped instead of blocking the critical logging path,
    // and the drop is counted for observability. This guarantees a stalled
    // listener can never apply backpressure to the caller or grow memory
    // without bound.
    if self
      .event_broadcast_sender
      .try_send(EventBroadcast { event })
      .is_err()
    {
      DROPPED_EVENTS.fetch_add(1, Ordering::Relaxed);
    }
  }

  // Separate writer loop focused only on snapshots and control messages
  fn writer_loop(
    receiver: Receiver<Message>,
    capacity: usize,
    service_name: String,
    storage_path: String,
    mut snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
    interner: Arc<StringInterner>,
  ) {
    let mut last_periodic = Instant::now();
    let periodic_flush_interval = Duration::seconds(60).to_std().unwrap();
    let service = SnapshotWriter::with_storage_path(service_name, storage_path);

    eprintln!(
      "[Trace] Writer thread started with buffer capacity: {}",
      capacity
    );

    loop {
      // Handle control messages with timeout to allow periodic snapshots
      match receiver.recv_timeout(periodic_flush_interval) {
        Ok(msg) => match msg {
          Message::SnapshotImmediate(reason, ack) => {
            eprintln!(
              "[Snapshot] Requested: {} (buffer has {} events)",
              reason,
              snapshot_buffer.len()
            );

            if !snapshot_buffer.is_empty() {
              if let Err(e) =
                service.snapshot_and_write(&mut snapshot_buffer, reason.clone(), interner.clone())
              {
                eprintln!("[Snapshot] failed: {}", e);
              } else {
                eprintln!("[Snapshot] completed successfully");
              }
            } else {
              eprintln!("[Snapshot] buffer empty, skipping snapshot");
            }
            let _ = ack.send(());
          },
          Message::FlushAndExit => {
            eprintln!("[Trace] Received shutdown signal");

            // Final snapshot with remaining events
            if !snapshot_buffer.is_empty() {
              let _ = service.snapshot_and_write(&mut snapshot_buffer, "flush_and_exit", interner);
            }

            eprintln!("[Trace] Writer thread shutting down");
            return;
          },
        },
        Err(_) => {
          // Timeout occurred - check for periodic snapshot
          if last_periodic.elapsed() >= periodic_flush_interval && !snapshot_buffer.is_empty() {
            eprintln!(
              "[Snapshot] Periodic snapshot triggered ({} events)",
              snapshot_buffer.len()
            );
            let _result =
              service.snapshot_and_write(&mut snapshot_buffer, "periodic", interner.clone());
            last_periodic = Instant::now();
          }
        },
      }
    }
  }

  // Dedicated listener loop - handles all listener events without limits
  fn listener_loop(
    listener_receiver: Receiver<ListenerMessage>,
    event_receiver: Receiver<EventBroadcast>,
    interner: Arc<StringInterner>,
  ) {
    let mut listeners: Vec<
      Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
    > = Vec::new();

    println!("[Trace] Listener thread started");

    loop {
      crossbeam_channel::select! {
        recv(listener_receiver) -> msg => {
          match msg {
            Ok(ListenerMessage::Add(listener, ack)) => {
              listener.on_start();
              listeners.push(listener);
              eprintln!("[Trace] Added new listener, total: {}", listeners.len());
              let _ = ack.send(());
            },
            Ok(ListenerMessage::Shutdown) => {
              eprintln!("[Trace] Listener thread received shutdown signal");

              // Process any remaining events
              while let Ok(event_broadcast) = event_receiver.try_recv() {
                for listener in &listeners {
                  let _ = std::panic::catch_unwind(|| {
                    listener.handle(&event_broadcast.event, &interner);
                  });
                }
              }

              // Cleanup listeners
              for listener in &listeners {
                listener.on_shutdown();
              }

              eprintln!("[Trace] Listener thread shutting down");
              return;
            },
            Err(_) => {
              // Channel closed
              eprintln!("[Trace] Listener management channel closed");
              return;
            }
          }
        },
        recv(event_receiver) -> event_msg => {
          match event_msg {
            Ok(event_broadcast) => {
              // Broadcast to ALL listeners - no limits, no batching
              for listener in &listeners {
                let result = std::panic::catch_unwind(|| {
                  listener.handle(&event_broadcast.event, &interner);
                });

                if result.is_err() {
                  eprintln!("[Trace] Listener panicked, continuing with others");
                }
              }
            },
            Err(_) => {
              // Event channel closed
              eprintln!("[Trace] Event broadcast channel closed");
              return;
            }
          }
        }
      }
    }
  }
}

impl Clone for Trace {
  fn clone(&self) -> Self {
    Self {
      snapshot_buffer: Arc::clone(&self.snapshot_buffer),
      sender: self.sender.clone(),
      event_broadcast_sender: self.event_broadcast_sender.clone(),
      level: AtomicU8::new(self.level.load(Ordering::Relaxed)),
      interner: Arc::clone(&self.interner),
      listener_sender: self.listener_sender.clone(),
      write_thread: None,
      listener_thread: None,
    }
  }
}

impl Drop for Trace {
  fn drop(&mut self) {
    // If shutdown wasn't called explicitly
    let _ = self.listener_sender.try_send(ListenerMessage::Shutdown);
    let _ = self.sender.try_send(Message::FlushAndExit);

    if let Some(handle) = self.write_thread.take() {
      let _ = handle.join();
    }
    if let Some(handle) = self.listener_thread.take() {
      let _ = handle.join();
    }
  }
}
