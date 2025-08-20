use std::{sync::Arc, thread, time::Duration};
use tracing_subscriber::{layer::SubscriberExt, Registry};
use ttlog::{
  event::{LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
  string_interner::StringInterner,
  trace::Message,
  trace_layer::{BatchedBufferLayer, BufferLayer},
};

/// High-performance logger configuration
#[derive(Debug, Clone)]
pub struct PerformanceConfig {
  /// Ring buffer capacity (higher = more memory, less drops)
  pub buffer_capacity: usize,
  /// Channel capacity between layer and writer
  pub channel_capacity: usize,
  /// Use batched layer for maximum throughput
  pub use_batching: bool,
  /// Writer thread priority boost
  pub high_priority_writer: bool,
  /// Periodic flush interval in seconds
  pub flush_interval_secs: u64,
}

impl Default for PerformanceConfig {
  fn default() -> Self {
    Self {
      buffer_capacity: 65536, // 64K events
      channel_capacity: 8192, // 8K message queue
      use_batching: true,     // Enable batching for max perf
      high_priority_writer: true,
      flush_interval_secs: 30, // More frequent flushes
    }
  }
}

/// High-performance trace system optimized for maximum throughput
#[derive(Debug)]
pub struct HighPerformanceTrace {
  sender: crossbeam_channel::Sender<Message>,
  interner: Arc<StringInterner>,
  config: PerformanceConfig,
  writer_handle: Option<thread::JoinHandle<()>>,
}

impl HighPerformanceTrace {
  /// Initialize high-performance logging system
  pub fn init(config: PerformanceConfig) -> Self {
    let (sender, receiver) = crossbeam_channel::bounded::<Message>(config.channel_capacity);
    let interner = Arc::new(StringInterner::new());

    // Spawn optimized writer thread
    let writer_interner = Arc::clone(&interner);
    let writer_config = config.clone();

    let writer_handle = thread::Builder::new()
      .name("ttlog-writer".to_string())
      .spawn(move || {
        Self::optimized_writer_loop(receiver, writer_config, writer_interner);
      })
      .expect("Failed to spawn writer thread");

    // Set thread priority if requested
    if config.high_priority_writer {
      // Note: This would require platform-specific code in a real implementation
      #[cfg(unix)]
      unsafe {
        libc::setpriority(libc::PRIO_PROCESS, 0, -10); // Higher priority
      }
    }

    // Create and register the appropriate layer
    let layer: Box<dyn tracing_subscriber::Layer<Registry> + Send + Sync> = if config.use_batching {
      Box::new(BatchedBufferLayer::new(
        sender.clone(),
        Arc::clone(&interner),
      ))
    } else {
      Box::new(BufferLayer::new(sender.clone(), Arc::clone(&interner)))
    };

    let subscriber = Registry::default().with(layer);
    let _ = tracing::subscriber::set_global_default(subscriber);

    Self {
      sender,
      interner,
      config,
      writer_handle: Some(writer_handle),
    }
  }

  /// Optimized writer loop with better performance characteristics
  fn optimized_writer_loop(
    receiver: crossbeam_channel::Receiver<Message>,
    config: PerformanceConfig,
    _interner: Arc<StringInterner>,
  ) {
    let mut ring = LockFreeRingBuffer::new(config.buffer_capacity);
    let mut last_flush = std::time::Instant::now();
    let flush_interval = Duration::from_secs(config.flush_interval_secs);

    // Pre-allocate batch processing buffer
    let mut batch = Vec::with_capacity(256);

    // Use select for better responsiveness
    loop {
      // Try to receive multiple messages in a batch
      match receiver.recv_timeout(Duration::from_millis(10)) {
        Ok(msg) => {
          match msg {
            Message::Event(event) => {
              let _ = ring.push(event);
            },
            Message::SnapshotImmediate { field1: reason } => {
              if !ring.is_empty() {
                Self::create_snapshot(&mut ring, &reason);
              }
            },
            Message::FlushAndExit => {
              if !ring.is_empty() {
                Self::create_snapshot(&mut ring, "shutdown");
              }
              break;
            },
          }

          // Try to batch process more messages without blocking
          while let Ok(msg) = receiver.try_recv() {
            match msg {
              Message::Event(event) => {
                let _ = ring.push(event);
              },
              Message::SnapshotImmediate { field1: reason } => {
                if !ring.is_empty() {
                  Self::create_snapshot(&mut ring, &reason);
                }
              },
              Message::FlushAndExit => {
                if !ring.is_empty() {
                  Self::create_snapshot(&mut ring, "shutdown");
                }
                return;
              },
            }

            // Limit batch size to avoid latency spikes
            if batch.len() >= 1000 {
              break;
            }
          }
        },
        Err(crossbeam_channel::RecvTimeoutError::Timeout) => {
          // Check for periodic flush
          if last_flush.elapsed() >= flush_interval && !ring.is_empty() {
            Self::create_snapshot(&mut ring, "periodic");
            last_flush = std::time::Instant::now();
          }
        },
        Err(crossbeam_channel::RecvTimeoutError::Disconnected) => {
          if !ring.is_empty() {
            Self::create_snapshot(&mut ring, "disconnected");
          }
          break;
        },
      }
    }
  }

  /// Fast snapshot creation
  fn create_snapshot(ring: &mut LockFreeRingBuffer<LogEvent>, reason: &str) {
    let events = ring.take_snapshot();
    if !events.is_empty() {
      // In a real implementation, this would use the SnapshotWriter
      eprintln!("[Snapshot] Captured {} events ({})", events.len(), reason);

      // For demonstration, just print stats
      let stats = ring.stats();
      eprintln!(
        "[Stats] Buffer: {}, Evicted: {}",
        stats.current_size, stats.total_evicted
      );
    }
  }

  /// Request immediate snapshot
  pub fn snapshot(&self, reason: &str) {
    let _ = self.sender.try_send(Message::SnapshotImmediate {
      field1: reason.to_string(),
    });
  }

  /// Get performance statistics
  pub fn stats(&self) -> (usize, usize, usize) {
    self.interner.stats()
  }

  /// Graceful shutdown
  pub fn shutdown(mut self) {
    // Send shutdown message
    let _ = self.sender.send(Message::FlushAndExit);

    // Wait for writer thread to finish
    if let Some(handle) = self.writer_handle.take() {
      let _ = handle.join();
    }
  }
}

/// Convenience function for quick setup with optimal defaults
pub fn init_high_performance_logging() -> HighPerformanceTrace {
  HighPerformanceTrace::init(PerformanceConfig::default())
}

/// Convenience function for maximum performance setup
pub fn init_maximum_performance_logging() -> HighPerformanceTrace {
  let config = PerformanceConfig {
    buffer_capacity: 131072, // 128K events
    channel_capacity: 16384, // 16K message queue
    use_batching: true,
    high_priority_writer: true,
    flush_interval_secs: 15, // Very frequent flushes
  };
  HighPerformanceTrace::init(config)
}

/// Convenience function for memory-optimized setup
pub fn init_memory_optimized_logging() -> HighPerformanceTrace {
  let config = PerformanceConfig {
    buffer_capacity: 8192,  // 8K events
    channel_capacity: 1024, // 1K message queue
    use_batching: false,    // Disable batching to save memory
    high_priority_writer: false,
    flush_interval_secs: 120, // Less frequent flushes
  };
  HighPerformanceTrace::init(config)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::time::Instant;

  #[test]
  fn test_high_performance_setup() {
    let trace = init_high_performance_logging();

    // Log some messages
    let start = Instant::now();
    for i in 0..10000 {
      tracing::info!(message = "Test message", count = i);
    }
    let elapsed = start.elapsed();

    println!("Logged 10,000 messages in {:?}", elapsed);
    println!("Rate: {:.0} msgs/sec", 10000.0 / elapsed.as_secs_f64());

    trace.snapshot("test_complete");
    trace.shutdown();
  }

  #[test]
  fn test_string_interner_performance() {
    let interner = Arc::new(StringInterner::new());

    let start = Instant::now();
    for i in 0..100000 {
      let target = format!("module::target::{}", i % 100); // Simulate repeated targets
      let message = format!("Message number {}", i);

      interner.intern_target(&target);
      interner.intern_message(&message);
    }
    let elapsed = start.elapsed();

    println!("Interned 200,000 strings in {:?}", elapsed);
    println!("Rate: {:.0} ops/sec", 200000.0 / elapsed.as_secs_f64());
    println!("Stats: {:?}", interner.stats());
  }

  #[test]
  fn test_ring_buffer_performance() {
    let buffer = LockFreeRingBuffer::new(10000);

    let start = Instant::now();
    for i in 0..1000000 {
      buffer.push_overwrite(LogEvent::new());
    }
    let elapsed = start.elapsed();

    println!("Pushed 1,000,000 events in {:?}", elapsed);
    println!("Rate: {:.0} ops/sec", 1000000.0 / elapsed.as_secs_f64());
    println!("Buffer stats: {:?}", buffer.stats());
  }
}

// Performance benchmarking utilities
pub mod benchmarks {
  use super::*;
  use std::time::Instant;

  pub fn benchmark_event_creation(count: usize) {
    let interner = Arc::new(StringInterner::new());

    let start = Instant::now();
    for i in 0..count {
      let target_id = interner.intern_target("benchmark::module");
      let message_id = interner.intern_message(&format!("Benchmark message {}", i));

      let mut event = LogEvent::new();
      event.packed_meta = LogEvent::pack_meta(
        chrono::Utc::now().timestamp_millis() as u64,
        LogLevel::INFO,
        1,
      );
      event.target_id = target_id;
      event.message_id = message_id;
    }
    let elapsed = start.elapsed();

    println!("Created {} events in {:?}", count, elapsed);
    println!(
      "Rate: {:.0} events/sec",
      count as f64 / elapsed.as_secs_f64()
    );
  }

  pub fn benchmark_full_pipeline(count: usize) {
    let trace = init_maximum_performance_logging();

    let start = Instant::now();
    for i in 0..count {
      tracing::info!(
        message = "Benchmark message",
        iteration = i,
        timestamp = chrono::Utc::now().timestamp()
      );
    }
    let elapsed = start.elapsed();

    println!("Full pipeline {} messages in {:?}", count, elapsed);
    println!("Rate: {:.0} msgs/sec", count as f64 / elapsed.as_secs_f64());

    trace.snapshot("benchmark_complete");
    thread::sleep(Duration::from_millis(100)); // Let snapshot complete
    trace.shutdown();
  }
}

fn main() {
  benchmarks::benchmark_full_pipeline(100000);
  benchmarks::benchmark_event_creation(100000);
  benchmarks::benchmark_full_pipeline(1000000);
  benchmarks::benchmark_event_creation(1000000);
}
