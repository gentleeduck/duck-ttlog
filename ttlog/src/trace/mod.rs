mod __test__;

use chrono::Duration;
use std::time::Instant;
use std::{sync::Arc, thread};
use tracing_subscriber::layer::SubscriberExt;

use crate::event::{LogEvent, LogLevel};
use crate::lf_buffer::LockFreeRingBuffer;
use crate::snapshot::SnapshotWriter;
use crate::string_interner::StringInterner;
use crate::trace_layer::BufferLayer;

use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic::{self, AtomicU8, Ordering};

#[derive(Debug)]
pub struct Trace {
  pub sender: Sender<Message>,
  pub level: atomic::AtomicU8,
}

#[derive(Debug)]
pub enum Message {
  Event(LogEvent),
  SnapshotImmediate(String), // reason
  FlushAndExit,              // optional: for graceful shutdown in tests
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
impl Trace {
  pub fn new(sender: Sender<Message>, interner: Arc<StringInterner>) -> Self {
    Self {
      sender,
      level: AtomicU8::new(LogLevel::INFO as u8),
      // interner,
    }
  }

  pub fn init(capacity: usize, channel_capacity: usize) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let interner = Arc::new(StringInterner::new());

    // Spawn writer thread which owns the ring buffer
    let writer_interner = Arc::clone(&interner);
    thread::spawn(move || Self::writer_loop(receiver, capacity, writer_interner));

    // Create and register BufferLayer using the sender
    let layer = BufferLayer::new(sender.clone(), Arc::clone(&interner));
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

    Self::new(sender, interner)
  }

  /// Returns a clone of the sender
  pub fn get_sender(&self) -> Sender<Message> {
    self.sender.clone()
  }

  /// Requests an immediate snapshot of the current ring buffer.
  pub fn request_snapshot(&self, reason: &str) {
    let _ = self
      .sender
      .try_send(Message::SnapshotImmediate(reason.to_string()));
  }

  /// Set the minimum log level
  pub fn set_level(&self, level: LogLevel) {
    self.level.store(level as u8, Ordering::Relaxed);
  }

  /// Get the current minimum log level
  pub fn get_level(&self) -> LogLevel {
    let level_u8 = self.level.load(Ordering::Relaxed);
    unsafe { std::mem::transmute(level_u8) }
  }

  /// The main writer loop that runs on a dedicated thread.
  fn writer_loop(
    receiver: crossbeam_channel::Receiver<Message>,
    capacity: usize,
    _interner: Arc<StringInterner>, // Available for future use
  ) {
    let mut ring = LockFreeRingBuffer::new(capacity);
    let mut last_periodic = Instant::now();
    let periodic_flush_interval = Duration::seconds(60).to_std().unwrap();

    let service = SnapshotWriter::new("ttlog");

    while let Ok(msg) = receiver.recv() {
      match msg {
        Message::Event(ev) => {
          let _ = ring.push(ev);
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
          if !ring.is_empty() {
            let _ = service.snapshot_and_write(&mut ring, "flush_and_exit");
          }
          break;
        },
      }

      // Periodic flush
      if last_periodic.elapsed() >= periodic_flush_interval && !ring.is_empty() {
        let _ = service.snapshot_and_write(&mut ring, "periodic");
        last_periodic = Instant::now();
      }
    }
  }
}
