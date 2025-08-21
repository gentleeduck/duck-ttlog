mod __test__;

use chrono::Duration;
use std::sync::OnceLock;
use std::time::Instant;
use std::{sync::Arc, thread};

use crate::event::{LogEvent, LogLevel};
use crate::lf_buffer::LockFreeRingBuffer;
use crate::listener::LogListener;
use crate::panic_hook::PanicHook;
use crate::snapshot::SnapshotWriter;
use crate::string_interner::StringInterner;
use crossbeam_channel::Sender;
use std::sync::atomic::{self, AtomicU8, Ordering};

#[derive(Debug)]
pub enum Message {
  SnapshotImmediate(&'static str),
  FlushAndExit,
}

pub enum ListenerMessage {
  Add(Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>),
  Shutdown,
}

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Message::SnapshotImmediate(reason) => write!(f, "SnapshotImmediate: {}", reason),
      Message::FlushAndExit => write!(f, "FlushAndExit"),
    }
  }
}

#[derive(Debug)]
pub struct Trace {
  /// For real-time listeners (gets drained continuously)
  pub listener_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  /// For snapshots (accumulates events, only drained on snapshot)
  pub snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  /// Channel sender for communicating with the writer thread
  pub sender: Sender<Message>,
  /// Atomic log level for runtime filtering
  pub level: atomic::AtomicU8,
  pub interner: Arc<StringInterner>,
  pub listener_sender: Sender<ListenerMessage>,
}

thread_local! {
  pub static GLOBAL_LOGGER: OnceLock<Trace> = OnceLock::new();
}

impl Trace {
  pub fn new(
    sender: Sender<Message>,
    listener_sender: Sender<ListenerMessage>,
    interner: Arc<StringInterner>,
    listener_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
    snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  ) -> Self {
    Self {
      sender,
      listener_buffer,
      snapshot_buffer,
      interner,
      listener_sender,
      level: AtomicU8::new(LogLevel::ERROR as u8),
    }
  }

  pub fn init(
    capacity: usize,
    channel_capacity: usize,
    service_name: &str,
    storage_path: Option<&str>,
  ) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let (listener_sender, listener_receiver) = crossbeam_channel::bounded::<ListenerMessage>(16);

    let interner = Arc::new(StringInterner::new());

    // Create separate buffers for listeners and snapshots
    let listener_buffer = Arc::new(LockFreeRingBuffer::new(capacity));
    let snapshot_buffer = Arc::new(LockFreeRingBuffer::new(capacity));

    let listener_buffer_clone = Arc::clone(&listener_buffer);
    let snapshot_buffer_clone = Arc::clone(&snapshot_buffer);
    let interner_clone = Arc::clone(&interner);

    // Install panic hook before spawning writer thread
    PanicHook::install(sender.clone());

    // Spawn writer thread with listener support
    let service_name = service_name.to_string();
    let storage_path = storage_path.ok_or("").unwrap().to_string();
    thread::spawn(move || {
      Self::writer_loop_with_listeners(
        receiver,
        listener_receiver,
        capacity,
        service_name,
        storage_path,
        listener_buffer_clone,
        snapshot_buffer_clone,
        interner_clone,
      )
    });

    let trace = Trace::new(
      sender,
      listener_sender,
      interner,
      listener_buffer,
      snapshot_buffer,
    );
    GLOBAL_LOGGER.with(|logger_cell| match logger_cell.set(trace.clone()) {
      Ok(_) => {},
      Err(_) => panic!("GLOBAL_LOGGER already initialized"),
    });

    trace
  }

  pub fn add_listener(
    &self,
    listener: Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
  ) {
    let _ = self
      .listener_sender
      .try_send(ListenerMessage::Add(listener));
  }

  pub fn shutdown(&self) {
    let _ = self.sender.try_send(Message::FlushAndExit);
    // Give the writer thread a moment to process the shutdown
    thread::sleep(Duration::milliseconds(100).to_std().unwrap());
  }

  pub fn get_sender(&self) -> Sender<Message> {
    self.sender.clone()
  }

  pub fn request_snapshot(&self, reason: &'static str) {
    eprintln!("[Snapshot Request] Captured snapshot request: {:?}", reason);

    // non-blocking attempt to enqueue; do NOT block in Snapshot Request handler
    if let Err(e) = self.sender.try_send(Message::SnapshotImmediate(reason)) {
      eprintln!(
        "[Snapshot Request] Unable to enqueue snapshot request: {:?}",
        e
      );
    } else {
      eprintln!("[Snapshot Request] Snapshot request enqueued");
    }

    // Give the writer thread time to process the snapshot
    thread::sleep(Duration::milliseconds(120).to_std().unwrap());

    eprintln!("[Snapshot Request] Snapshot request completed");
  }

  pub fn set_level(&self, level: LogLevel) {
    self.level.store(level as u8, Ordering::Relaxed);
  }

  pub fn get_level(&self) -> LogLevel {
    let level_u8 = self.level.load(Ordering::Relaxed);
    unsafe { std::mem::transmute(level_u8) }
  }

  #[inline(always)]
  pub fn send_event_fast(
    &self,
    log_level: u8,
    target_id: u16,
    message_id: Option<u16>,
    thread_id: u8,
    file_id: u16,
    position: (u32, u32),
    kv_id: Option<u16>,
  ) {
    // Fast level check first
    if log_level > self.level.load(Ordering::Relaxed) {
      return;
    }

    // Create event directly on stack - no heap allocation
    let timestamp = unsafe {
      // SAFETY: This is faster than chrono for hot path
      // Use rdtsc or similar for even faster timing if needed
      // TODO: support other platforms
      std::arch::x86_64::_rdtsc()
    };

    let event = LogEvent {
      packed_meta: LogEvent::pack_meta(
        timestamp,
        unsafe { std::mem::transmute(log_level) },
        thread_id,
      ),
      target_id,
      message_id,
      position,
      file_id,
      kv_id,
      _padding: [0; 9],
    };

    // Push to both buffers - listeners get real-time events, snapshots accumulate
    self.listener_buffer.push_overwrite(event.clone());
    self.snapshot_buffer.push_overwrite(event);
  }

  fn writer_loop_with_listeners(
    receiver: crossbeam_channel::Receiver<Message>,
    listener_receiver: crossbeam_channel::Receiver<ListenerMessage>,
    capacity: usize,
    service_name: String,
    storage_path: String,
    listener_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
    mut snapshot_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
    interner: Arc<StringInterner>,
  ) {
    let mut last_periodic = Instant::now();
    let periodic_flush_interval = Duration::seconds(60).to_std().unwrap();
    let service = SnapshotWriter::new(service_name, storage_path);

    // Listener management
    let mut listeners: Vec<
      Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
    > = Vec::new();

    eprintln!(
      "[Trace] Writer thread started with buffer capacity: {}",
      capacity
    );

    // Main event loop - handles both log events and listener management
    loop {
      // Handle control messages first
      if let Ok(msg) = receiver.try_recv() {
        match msg {
          Message::SnapshotImmediate(reason) => {
            eprintln!(
              "[Snapshot] Requested: {} (buffer has {} events)",
              reason,
              snapshot_buffer.len()
            );

            if !snapshot_buffer.is_empty() {
              if let Err(e) =
                service.snapshot_and_write(&mut snapshot_buffer, reason, interner.clone())
              {
                eprintln!("[Snapshot] failed: {}", e);
              } else {
                eprintln!("[Snapshot] completed successfully");
              }
            } else {
              eprintln!("[Snapshot] buffer empty, skipping snapshot");
            }
          },
          Message::FlushAndExit => {
            eprintln!("[Trace] Received shutdown signal");

            // Final flush - process remaining listener events
            while let Some(event) = listener_buffer.pop() {
              for listener in &listeners {
                let _ = std::panic::catch_unwind(|| {
                  listener.handle(&event, &interner);
                });
              }
            }

            // Cleanup listeners
            for listener in &listeners {
              listener.on_shutdown();
            }

            // Final snapshot with remaining events
            if !snapshot_buffer.is_empty() {
              let _ = service.snapshot_and_write(&mut snapshot_buffer, "flush_and_exit", interner);
            }

            eprintln!("[Trace] Writer thread shutting down");
            return;
          },
        }
      }

      // Check for new listeners (non-blocking)
      while let Ok(listener_msg) = listener_receiver.try_recv() {
        match listener_msg {
          ListenerMessage::Add(listener) => {
            listener.on_start();
            listeners.push(listener);
            eprintln!("[Trace] Added new listener, total: {}", listeners.len());
          },
          ListenerMessage::Shutdown => {
            for listener in &listeners {
              listener.on_shutdown();
            }
            return;
          },
        }
      }

      // Process log events for listeners - drain listener buffer
      let mut events_processed = 0;
      while let Some(event) = listener_buffer.pop() {
        // Fanout to all listeners - isolated panic handling
        for listener in &listeners {
          let result = std::panic::catch_unwind(|| {
            listener.handle(&event, &interner);
          });

          if result.is_err() {
            eprintln!("[Trace] Listener panicked, continuing with others");
          }
        }

        events_processed += 1;

        // Batch processing - don't monopolize the thread
        if events_processed >= 100 {
          break;
        }
      }

      // Periodic snapshot (keep existing behavior) - use snapshot buffer
      if last_periodic.elapsed() >= periodic_flush_interval && !snapshot_buffer.is_empty() {
        eprintln!(
          "[Snapshot] Periodic snapshot triggered ({} events)",
          snapshot_buffer.len()
        );
        let _result =
          service.snapshot_and_write(&mut snapshot_buffer, "periodic", interner.clone());
        last_periodic = Instant::now();
      }

      // Small yield to prevent busy waiting when no events
      if events_processed == 0 {
        thread::sleep(std::time::Duration::from_micros(100));
      }
    }
  }
}

impl Clone for Trace {
  fn clone(&self) -> Self {
    Self {
      listener_buffer: Arc::clone(&self.listener_buffer),
      snapshot_buffer: Arc::clone(&self.snapshot_buffer),
      sender: self.sender.clone(),
      level: AtomicU8::new(self.level.load(Ordering::Relaxed)),
      interner: Arc::clone(&self.interner),
      listener_sender: self.listener_sender.clone(),
    }
  }
}
