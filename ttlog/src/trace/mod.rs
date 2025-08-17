mod __test__;

use chrono::Duration;
use std::thread;
use std::time::Instant;
use tracing_subscriber::layer::SubscriberExt;

use crate::event::LogEvent;
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;
use crate::snapshot::SnapshotWriter;
use crate::trace_layer::BufferLayer;

use crossbeam_channel::{bounded, Receiver, Sender};
use std::sync::atomic;

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
  pub fn new(sender: Sender<Message>) -> Self {
    Self {
      sender,
      level: atomic::AtomicU8::new(0),
    }
  }

  pub fn init(capacity: usize, channel_capacity: usize) -> Self {
    let (sender, receiver) = bounded::<Message>(channel_capacity);

    // Spawn writer thread which owns the ring buffer
    thread::spawn(move || Trace::writer_loop(receiver, capacity));

    // Create and register BufferLayer using the sender
    let layer = BufferLayer::new(sender.clone());
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let _ = tracing::subscriber::set_global_default(subscriber); // ignore error if already set

    Self {
      sender,
      level: atomic::AtomicU8::new(0),
    }
  }

  /// Returns a clone of the sender used to send messages into the tracing buffer.
  ///
  /// This allows other threads or components to asynchronously send `Message`s
  /// (events or snapshot requests) to the writer thread.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::trace::{Trace, Message};
  /// use ttlog::trace::Message::Event;
  ///
  /// ```
  pub fn get_sender(&self) -> Sender<Message> {
    self.sender.clone()
  }

  /// Requests an immediate snapshot of the current ring buffer.
  ///
  /// Sends a `SnapshotImmediate` message into the channel. The `reason` is included
  /// in the snapshot metadata for logging or debugging purposes.
  ///
  /// If the channel is full, the request is ignored.
  ///
  /// # Parameters
  /// - `reason`: A string describing why the snapshot was requested.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::trace::Trace;
  ///
  /// let trace_system = Trace::init(1024, 128);
  /// trace_system.request_snapshot("manual_debug_snapshot");
  /// ```
  pub fn request_snapshot(&self, reason: &str) {
    let _ = self
      .sender
      .try_send(Message::SnapshotImmediate(reason.to_string()));
  }

  /// The main writer loop that runs on a dedicated thread.
  ///
  /// This function continuously receives messages from the channel and:
  /// - Stores events in a ring buffer.
  /// - Writes immediate snapshots when requested.
  /// - Flushes and exits when requested.
  /// - Performs periodic flushes every 60 seconds.
  ///
  /// # Parameters
  /// - `receiver`: The channel receiver used to receive messages from other threads.
  /// - `capacity`: The size of the ring buffer to store incoming events.
  ///
  /// # Notes
  /// - This function is intended to run on a separate thread.
  /// - Snapshots are written using `snapshot_and_write`.
  fn writer_loop(receiver: Receiver<Message>, capacity: usize) {
    let mut ring = RingBuffer::new(capacity);
    let mut last_periodic = Instant::now();
    // you can set a periodic flush interval
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
            eprintln!(
              "[Snapshot] buffer empty, skipping snapshot (reason={})",
              reason
            );
          }
        },
        Message::FlushAndExit => {
          if !ring.is_empty() {
            let _ = service.snapshot_and_write(&mut ring, "flush_and_exit".to_string());
          }
          break;
        },
      }

      // periodic flush
      if last_periodic.elapsed() >= periodic_flush_interval && !ring.is_empty() {
        let _ = service.snapshot_and_write(&mut ring, "periodic".to_string());
        last_periodic = Instant::now();
      }
    }
  }
}
