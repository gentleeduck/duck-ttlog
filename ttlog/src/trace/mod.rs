mod __test__;

use chrono::Duration;
use std::sync::OnceLock;
use std::time::Instant;
use std::{sync::Arc, thread};

use crate::event::{LogEvent, LogLevel};
use crate::lf_buffer::LockFreeRingBuffer;
use crate::panic_hook::PanicHook;
use crate::snapshot::SnapshotWriter;
use crate::string_interner::StringInterner;
use crossbeam_channel::Sender;
use std::sync::atomic::{self, AtomicU8, Ordering};

/// Messages sent between the logging layer and the writer thread.
///
/// This enum encapsulates all communication between the producer side (logging calls)
/// and the consumer side (writer thread that manages the ring buffer).
#[derive(Debug)]
pub enum Message {
  /// A log event to be stored in the ring buffer
  Event(LogEvent),
  /// Request for immediate snapshot with a reason string
  SnapshotImmediate(&'static str),
  /// Signal for graceful shutdown with final flush
  FlushAndExit,
}

impl std::fmt::Display for Message {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Message::Event(ev) => write!(f, "Event: {}", ev),
      Message::SnapshotImmediate(reason) => write!(f, "SnapshotImmediate: {}", reason),
      Message::FlushAndExit => write!(f, "FlushAndExit"),
    }
  }
}

#[derive(Debug)]
pub struct Trace {
  /// Direct reference to ring buffer for zero-copy fast path
  pub ring_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  /// Channel sender for communicating with the writer thread
  pub sender: Sender<Message>,
  /// Atomic log level for runtime filtering
  pub level: atomic::AtomicU8,
  pub interner: Arc<StringInterner>,
}

thread_local! {
  pub static GLOBAL_LOGGER: OnceLock<Trace> = OnceLock::new();
}

impl Trace {
  pub fn new(
    sender: Sender<Message>,
    interner: Arc<StringInterner>,
    ring_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  ) -> Self {
    Self {
      sender,
      ring_buffer,
      interner,
      level: AtomicU8::new(LogLevel::INFO as u8),
    }
  }

  pub fn init(capacity: usize, channel_capacity: usize, service_name: &str) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let interner = Arc::new(StringInterner::new());
    let ring_buffer = Arc::new(LockFreeRingBuffer::new(capacity));

    // Install panic hook before spawning writer thread
    PanicHook::install(sender.clone());

    // Spawn writer thread which owns the ring buffer
    let writer_interner = Arc::clone(&interner);
    let service_name = service_name.to_string();
    thread::spawn(move || Self::writer_loop(receiver, capacity, writer_interner, service_name));

    let trace = Trace::new(sender, interner, ring_buffer);
    GLOBAL_LOGGER.with(|logger_cell| match logger_cell.set(trace.clone()) {
      Ok(_) => {},
      Err(_) => panic!("GLOBAL_LOGGER already initialized"),
    });

    trace
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
    // Remove string allocation
    let _ = self.sender.try_send(Message::SnapshotImmediate(reason));
  }

  pub fn set_level(&self, level: LogLevel) {
    self.level.store(level as u8, Ordering::Relaxed);
  }

  pub fn get_level(&self) -> LogLevel {
    let level_u8 = self.level.load(Ordering::Relaxed);
    unsafe { std::mem::transmute(level_u8) }
  }

  #[inline(always)]
  pub fn send_event_fast(&self, log_level: u8, target_id: u16, message_id: u16, thread_id: u8) {
    // Fast level check first
    if log_level < self.level.load(Ordering::Relaxed) {
      return;
    }

    // Create event directly on stack - no heap allocation
    let timestamp = unsafe {
      // SAFETY: This is faster than chrono for hot path
      // Use rdtsc or similar for even faster timing if needed
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
      field_count: 0,
      fields: [crate::event::Field::empty(); 3],
      file_id: 0,
      line: 0,
      _padding: [0; 9],
    };

    // Direct ring buffer push - bypass channel for hot path
    self.ring_buffer.push_overwrite(event);
  }

  fn writer_loop(
    receiver: crossbeam_channel::Receiver<Message>,
    capacity: usize,
    interner: Arc<StringInterner>, // Keep reference for potential future use
    service_name: String,
  ) {
    let mut ring = LockFreeRingBuffer::new(capacity);
    let mut last_periodic = Instant::now();
    let periodic_flush_interval = Duration::seconds(60).to_std().unwrap();
    let service = SnapshotWriter::new(service_name);

    eprintln!(
      "[Trace] Writer thread started with buffer capacity: {}",
      capacity
    );

    while let Ok(msg) = receiver.recv() {
      match msg {
        Message::Event(ev) => {
          // Store event in ring buffer (may overwrite oldest on overflow)
          let _result = ring.push(ev);
        },
        Message::SnapshotImmediate(reason) => {
          if !ring.is_empty() {
            if let Err(e) = service.snapshot_and_write(&mut ring, reason) {
              eprintln!("[Snapshot] failed: {}", e);
            }
          } else {
            eprintln!("[Snapshot] buffer empty, skipping snapshot");
          }
        },
        Message::FlushAndExit => {
          eprintln!("[Trace] Received shutdown signal, performing final flush");
          // Final flush before shutdown
          if !ring.is_empty() {
            let _result = service.snapshot_and_write(&mut ring, "flush_and_exit");
          }
          eprintln!("[Trace] Writer thread shutting down");
          break;
        },
      }

      // Periodic flush every 60 seconds
      if last_periodic.elapsed() >= periodic_flush_interval && !ring.is_empty() {
        let _result = service.snapshot_and_write(&mut ring, "periodic");
        last_periodic = Instant::now();
      }
    }

    eprintln!("[Trace] Writer thread terminated");
  }
}

impl Clone for Trace {
  fn clone(&self) -> Self {
    Self {
      ring_buffer: Arc::clone(&self.ring_buffer),
      sender: self.sender.clone(),
      level: AtomicU8::new(self.level.load(Ordering::Relaxed)),
      interner: Arc::clone(&self.interner),
    }
  }
}
