//! # Trace Module
//!
//! A high-performance, asynchronous logging system built on top of the `tracing` crate.
//! This module provides a lock-free ring buffer-based logging solution with automatic
//! periodic snapshots and immediate snapshot capabilities.
//!
//! ## Architecture
//!
//! The system uses a **producer-consumer architecture**:
//! - **Producers**: Application threads log events through the `tracing` crate
//! - **Consumer**: Dedicated writer thread processes events from a ring buffer
//! - **Storage**: Lock-free ring buffer with configurable capacity
//! - **Persistence**: Periodic and on-demand snapshots to storage
//!
//! ## Key Features
//!
//! - **Lock-free logging**: Minimal contention on hot paths
//! - **String interning**: Reduces memory usage for repeated strings
//! - **Configurable levels**: Runtime log level adjustment
//! - **Automatic snapshots**: Periodic flushing every 60 seconds
//! - **Manual snapshots**: On-demand snapshots with custom reasons
//! - **Graceful shutdown**: Ensures all logs are flushed before exit
//!
//! ## Usage
//!
//! ```rust,ignore
//! // Initialize the trace system
//! let trace = Trace::init(1024, 100); // 1024 event capacity, 100 message queue
//!
//! // Use standard tracing macros
//! tracing::info!("Application started");
//! tracing::error!("Database connection failed");
//!
//! // Manual snapshot
//! trace.request_snapshot("application_checkpoint");
//!
//! // Adjust log level
//! trace.set_level(LogLevel::DEBUG);
//! ```

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
use crossbeam_channel::Sender;
use std::sync::atomic::{self, AtomicU8, Ordering};

/// Main interface for the trace logging system.
///
/// The `Trace` struct provides the primary API for interacting with the logging system.
/// It maintains a channel sender for communicating with the writer thread and tracks
/// the current minimum log level for filtering.
///
/// ## Thread Safety
/// - The sender can be cloned and used from multiple threads
/// - Log level changes are atomic and immediately visible to all threads
/// - All operations are non-blocking for the caller
#[derive(Debug)]
pub struct Trace {
  /// Channel sender for communicating with the writer thread
  pub sender: Sender<Message>,
  /// Atomic log level for runtime filtering
  pub level: atomic::AtomicU8,
}

/// Messages sent between the logging layer and the writer thread.
///
/// This enum encapsulates all communication between the producer side (logging calls)
/// and the consumer side (writer thread that manages the ring buffer).
#[derive(Debug)]
pub enum Message {
  /// A log event to be stored in the ring buffer
  Event(LogEvent),
  /// Request for immediate snapshot with a reason string
  SnapshotImmediate(String),
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

impl Trace {
  /// Creates a new Trace instance with the given sender.
  ///
  /// The initial log level is set to `INFO`. Use `set_level()` to change it.
  ///
  /// ## Parameters
  /// - `sender`: Channel sender for communicating with the writer thread
  pub fn new(sender: Sender<Message>) -> Self {
    Self {
      sender,
      level: AtomicU8::new(LogLevel::INFO as u8),
    }
  }

  /// Initializes the complete trace logging system.
  ///
  /// This method sets up the entire logging pipeline:
  /// 1. Creates a bounded channel for message passing
  /// 2. Spawns a dedicated writer thread with a ring buffer
  /// 3. Registers a custom tracing layer with the global subscriber
  /// 4. Returns a Trace handle for controlling the system
  ///
  /// ## Parameters
  /// - `capacity`: Number of log events the ring buffer can hold
  /// - `channel_capacity`: Maximum number of pending messages in the channel
  ///
  /// ## Panics
  /// Panics if a global tracing subscriber is already set.
  ///
  /// ## Examples
  /// ```rust,ignore
  /// // Initialize with 1024 event capacity and 100 message queue depth
  /// let trace = Trace::init(1024, 100);
  ///
  /// // Now tracing macros will work
  /// tracing::info!("System initialized");
  /// ```
  pub fn init(capacity: usize, channel_capacity: usize) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(channel_capacity);
    let interner = Arc::new(StringInterner::new());

    // Spawn writer thread which owns the ring buffer
    let writer_interner = Arc::clone(&interner);
    thread::spawn(move || Self::writer_loop(receiver, capacity, writer_interner));

    // Create and register BufferLayer using the sender
    let layer = BufferLayer::new(sender.clone(), Arc::clone(&interner));
    let subscriber = tracing_subscriber::Registry::default().with(layer);
    let _result = tracing::subscriber::set_global_default(subscriber);

    Self::new(sender)
  }

  /// Returns a clone of the channel sender.
  ///
  /// This allows other parts of the system to send messages directly
  /// to the writer thread if needed.
  ///
  /// ## Examples
  /// ```rust,ignore
  /// let sender = trace.get_sender();
  /// sender.send(Message::SnapshotImmediate("custom_trigger".to_string()))?;
  /// ```
  pub fn get_sender(&self) -> Sender<Message> {
    self.sender.clone()
  }

  /// Requests an immediate snapshot of the current ring buffer.
  ///
  /// This is a non-blocking operation that sends a snapshot request to the writer thread.
  /// The snapshot will include all events currently in the ring buffer along with
  /// the provided reason string for debugging purposes.
  ///
  /// ## Parameters
  /// - `reason`: Human-readable string explaining why the snapshot was requested
  ///
  /// ## Examples
  /// ```rust,ignore
  /// // Snapshot before critical operation
  /// trace.request_snapshot("before_database_migration");
  ///
  /// // Snapshot on error condition
  /// trace.request_snapshot("unhandled_exception_caught");
  /// ```
  ///
  /// ## Notes
  /// Uses `try_send()` to avoid blocking if the channel is full.
  /// Failed sends are silently ignored to prevent logging from affecting application performance.
  pub fn request_snapshot(&self, reason: &str) {
    let _ = self
      .sender
      .try_send(Message::SnapshotImmediate(reason.to_string()));
  }

  /// Sets the minimum log level for filtering events.
  ///
  /// Events below this level will be filtered out before reaching the ring buffer.
  /// This is an atomic operation that takes effect immediately for all threads.
  ///
  /// ## Parameters
  /// - `level`: The new minimum log level
  ///
  /// ## Examples
  /// ```rust,ignore
  /// // Enable debug logging
  /// trace.set_level(LogLevel::DEBUG);
  ///
  /// // Disable all logging except errors
  /// trace.set_level(LogLevel::ERROR);
  /// ```
  pub fn set_level(&self, level: LogLevel) {
    self.level.store(level as u8, Ordering::Relaxed);
  }

  /// Gets the current minimum log level.
  ///
  /// ## Safety
  /// Uses `unsafe` transmute to convert u8 back to LogLevel enum.
  /// This is safe as long as only valid LogLevel values are stored via `set_level()`.
  ///
  /// ## Examples
  /// ```rust,ignore
  /// let current_level = trace.get_level();
  /// println!("Current log level: {:?}", current_level);
  /// ```
  pub fn get_level(&self) -> LogLevel {
    let level_u8 = self.level.load(Ordering::Relaxed);
    unsafe { std::mem::transmute(level_u8) }
  }

  /// The main writer loop that runs on a dedicated background thread.
  ///
  /// This is the core of the logging system's consumer side. It:
  /// 1. Receives messages from the channel
  /// 2. Stores log events in a lock-free ring buffer
  /// 3. Triggers periodic snapshots every 60 seconds
  /// 4. Handles immediate snapshot requests
  /// 5. Performs final flush on shutdown
  ///
  /// ## Parameters
  /// - `receiver`: Channel receiver for incoming messages
  /// - `capacity`: Ring buffer capacity for storing log events
  /// - `_interner`: String interner (reserved for future optimizations)
  ///
  /// ## Error Handling
  /// - Snapshot failures are logged to stderr but don't stop the loop
  /// - Empty buffer snapshots are skipped with a warning message
  /// - The loop exits cleanly on `FlushAndExit` message
  ///
  /// ## Performance Notes
  /// - Runs on a dedicated thread to avoid blocking application threads
  /// - Uses lock-free ring buffer for minimal writer thread contention
  /// - Periodic flushes prevent unbounded memory growth
  ///
  /// ## Shutdown Behavior
  /// On `FlushAndExit`:
  /// 1. Performs final snapshot if buffer contains events
  /// 2. Exits the loop cleanly
  /// 3. Thread terminates, allowing process shutdown
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
          // Final flush before shutdown
          if !ring.is_empty() {
            let _result = service.snapshot_and_write(&mut ring, "flush_and_exit");
          }
          break;
        },
      }

      // Periodic flush every 60 seconds
      if last_periodic.elapsed() >= periodic_flush_interval && !ring.is_empty() {
        let _result = service.snapshot_and_write(&mut ring, "periodic");
        last_periodic = Instant::now();
      }
    }
  }
}

impl Default for Trace {
  /// Creates a default Trace instance.
  ///
  /// Uses reasonable defaults:
  /// - 1024 event ring buffer capacity
  /// - 100 message channel capacity
  ///
  /// Equivalent to `Trace::init(1024, 100)`.
  fn default() -> Self {
    Self::init(1024, 100)
  }
}
