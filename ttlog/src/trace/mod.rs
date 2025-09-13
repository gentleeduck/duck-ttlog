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
use crossbeam_channel::{unbounded, Receiver, Sender};
use std::sync::atomic::{self, AtomicU8, Ordering};

#[derive(Debug)]
pub enum Message {
  SnapshotImmediate(String, std::sync::mpsc::Sender<()>),
  FlushAndExit,
}

pub enum ListenerMessage {
  Add(Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>),
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
  /// Direct event broadcasting channel - unbounded to ensure no events are lost
  pub event_broadcast_sender: Sender<EventBroadcast>,
  /// Atomic log level for runtime filtering
  pub level: atomic::AtomicU8,
  pub interner: Arc<StringInterner>,
  pub listener_sender: Sender<ListenerMessage>,
  pub write_thread: Option<thread::JoinHandle<()>>,
  pub listener_thread: Option<thread::JoinHandle<()>>,
}

thread_local! {
  pub static GLOBAL_LOGGER: OnceLock<Trace> = OnceLock::new();
}

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

  pub fn init(
    capacity: usize,
    channel_capacity: usize,
    service_name: &str,
    storage_path: Option<&str>,
  ) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let (listener_sender, listener_receiver) = crossbeam_channel::bounded::<ListenerMessage>(16);

    // Use unbounded channel for event broadcasting to ensure no events are lost
    let (event_broadcast_sender, event_broadcast_receiver) = unbounded::<EventBroadcast>();

    let interner = Arc::new(StringInterner::new());

    // Only need snapshot buffer now - listeners get events directly
    let snapshot_buffer = Arc::new(LockFreeRingBuffer::new(capacity));
    let snapshot_buffer_clone = Arc::clone(&snapshot_buffer);
    let interner_clone = Arc::clone(&interner);

    // Install panic hook before spawning writer thread
    PanicHook::install(sender.clone());

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

    // Set the global logger BEFORE spawning the writer thread
    GLOBAL_LOGGER.with(|logger_cell| match logger_cell.set(trace.clone()) {
      Ok(_) => {
        println!("GLOBAL_LOGGER initialized");
      },
      Err(_) => panic!("GLOBAL_LOGGER already initialized"),
    });

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

    // Wait for writer and listener threads to start
    // write_thread_handle.join().unwrap();
    // listener_thread_handle.join().unwrap();

    trace
  }

  pub fn add_listener(
    &self,
    listener: Arc<dyn LogListener + std::panic::UnwindSafe + std::panic::RefUnwindSafe>,
  ) {
    if let Err(e) = self
      .listener_sender
      .try_send(ListenerMessage::Add(listener))
    {
      eprintln!("[Trace] Failed to add listener: {:?}", e);
    } else {
      println!("[Trace] Listener addition request sent");
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
    GLOBAL_LOGGER.with(|logger_cell| {
      if let Some(logger) = logger_cell.get() {
        logger.level.store(level as u8, Ordering::Relaxed);
      }
    });
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
        unsafe { std::mem::transmute(log_level) },
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

    // Broadcast to all listeners immediately - no buffering, no limits
    // Using unbounded channel ensures no events are lost
    if let Err(_) = self
      .event_broadcast_sender
      .try_send(EventBroadcast { event })
    {
      // If the channel is somehow full or closed, we could optionally log this
      // but we don't want to block the critical logging path
      eprintln!("[Trace] Warning: Failed to broadcast event to listeners");
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
    let service = SnapshotWriter::new(service_name, storage_path);

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

    eprintln!("[Trace] Listener thread started");

    loop {
      crossbeam_channel::select! {
        recv(listener_receiver) -> msg => {
          match msg {
            Ok(ListenerMessage::Add(listener)) => {
              listener.on_start();
              listeners.push(listener);
              eprintln!("[Trace] Added new listener, total: {}", listeners.len());
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
