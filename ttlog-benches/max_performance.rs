use std::fmt::Write as _;
use std::fs::{self, File};
use std::io::{BufWriter, Write};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};

use chrono::Utc;
use crossbeam_channel::{bounded, unbounded, RecvTimeoutError};
use tabled::{Table, Tabled};
use ttlog::{
  event::{LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
  string_interner::StringInterner,
  trace::{EventBroadcast, ListenerMessage, Message, Trace},
};

// ============================================================================
// Unified Test Result Types
// ============================================================================

#[derive(Debug, Clone, Tabled)]
struct TestResult {
  #[tabled(rename = "Test Name")]
  test_name: String,
  #[tabled(rename = "Metric")]
  metric: String,
  #[tabled(rename = "Value")]
  value: f64,
  #[tabled(rename = "Unit")]
  unit: String,
  #[tabled(rename = "Duration")]
  duration: String,
  #[tabled(rename = "Config")]
  config: String,
  #[tabled(rename = "Notes")]
  notes: String,
}

#[derive(Debug, Clone, Tabled)]
struct StatBufferTest {
  #[tabled(rename = "Test")]
  test_name: String,
  #[tabled(rename = "Producers")]
  producers: usize,
  #[tabled(rename = "Consumers")]
  consumers: usize,
  #[tabled(rename = "Buffer Size")]
  buffer_size: usize,
  #[tabled(rename = "Mean Ops/Sec")]
  mean_ops_per_sec: f64,
  #[tabled(rename = "StdDev")]
  stddev_ops_per_sec: f64,
  #[tabled(rename = "Runs")]
  runs: usize,
  #[tabled(rename = "Notes")]
  notes: String,
}

impl StatBufferTest {
  fn from_results(results: &[BufferTest]) -> Self {
    if results.is_empty() {
      return Self {
        test_name: "Empty".to_string(),
        producers: 0,
        consumers: 0,
        buffer_size: 0,
        mean_ops_per_sec: 0.0,
        stddev_ops_per_sec: 0.0,
        runs: 0,
        notes: "No results".to_string(),
      };
    }

    let values: Vec<f64> = results.iter().map(|r| r.ops_per_sec).collect();
    let (mean, stddev) = mean_std(&values);
    let first = &results[0];

    Self {
      test_name: first.test_name.clone(),
      producers: first.producers,
      consumers: first.consumers,
      buffer_size: first.buffer_size,
      mean_ops_per_sec: mean,
      stddev_ops_per_sec: stddev,
      runs: results.len(),
      notes: format!(
        "{} | Range: {:.0}-{:.0}",
        first.notes,
        values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
      ),
    }
  }
}

#[derive(Debug, Clone, Tabled)]
struct BufferTest {
  #[tabled(rename = "Test")]
  test_name: String,
  #[tabled(rename = "Producers")]
  producers: usize,
  #[tabled(rename = "Consumers")]
  consumers: usize,
  #[tabled(rename = "Buffer Size")]
  buffer_size: usize,
  #[tabled(rename = "Ops/Sec")]
  ops_per_sec: f64,
  #[tabled(rename = "Total Ops")]
  total_ops: u64,
  #[tabled(rename = "Duration")]
  duration: String,
  #[tabled(rename = "Notes")]
  notes: String,
}

#[allow(dead_code)]
#[derive(Debug, Clone, Tabled)]
struct SummaryMetric {
  #[tabled(rename = "Metric")]
  metric: String,
  #[tabled(rename = "Value")]
  value: f64,
  #[tabled(rename = "Unit")]
  unit: String,
}

// ============================================================================
// Throughput Tester
// ============================================================================

struct ThroughputTester {
  test_duration: Duration,
}

impl ThroughputTester {
  fn new(test_duration: Duration) -> Self {
    Self { test_duration }
  }

  /// Run throughput tests with multiple trials for statistical confidence
  fn run_throughput_tests(
    &self,
    buffer_size: usize,
    runs: usize,
  ) -> (StatTestResult, StatTestResult) {
    let events_results = run_multiple_times(
      || self.max_events_per_second(buffer_size),
      runs,
      Duration::from_millis(200),
    );

    let buffer_results = run_multiple_times(
      || self.max_buffer_operations(buffer_size),
      runs,
      Duration::from_millis(200),
    );

    (
      StatTestResult::from_results(&events_results),
      StatTestResult::from_results(&buffer_results),
    )
  }

  /// Test maximum events per second
  fn max_events_per_second(&self, buffer_size: usize) -> TestResult {
    let thread_count = thread::available_parallelism()
      .map(|n| n.get())
      .unwrap_or(8)
      .max(1);
    let event_count = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));

    let (msg_tx, _msg_rx) = bounded::<Message>(32);
    let (listener_tx, _listener_rx) = bounded::<ListenerMessage>(16);
    let (event_tx, event_rx) = unbounded::<EventBroadcast>();

    let logger = Arc::new(Trace::new(
      msg_tx,
      listener_tx,
      event_tx,
      Arc::new(StringInterner::new()),
      Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size)),
    ));
    logger.level.store(LogLevel::TRACE as u8, Ordering::Relaxed);

    let target_id = logger.interner.intern_target("bench::real_path");
    let file_id = logger.interner.intern_file(file!());
    let message_id = std::num::NonZeroU16::new(logger.interner.intern_message("bench event"));

    let drain_stop = Arc::clone(&stop_flag);
    let drain_handle = thread::spawn(move || {
      let mut drained = 0u64;
      loop {
        match event_rx.recv_timeout(Duration::from_millis(10)) {
          Ok(_) => drained += 1,
          Err(RecvTimeoutError::Timeout) => {
            if drain_stop.load(Ordering::Acquire) {
              while event_rx.try_recv().is_ok() {
                drained += 1;
              }
              return drained;
            }
          },
          Err(RecvTimeoutError::Disconnected) => return drained,
        }
      }
    });

    let end = Instant::now() + self.test_duration;
    let handles: Vec<_> = (0..thread_count)
      .map(|tid| {
        let logger = Arc::clone(&logger);
        let event_count = Arc::clone(&event_count);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);

        thread::spawn(move || {
          let mut local = 0u64;
          let thread_id = (tid & 0xFF) as u8;
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            logger.send_event_fast(
              LogLevel::INFO as u8,
              target_id,
              message_id,
              thread_id,
              file_id,
              ((local & 0xFFFF_FFFF) as u32, 0),
              None,
            );
            local += 1;
            if (local & 0x3FF) == 0 {
              thread::yield_now();
            }
          }
          event_count.fetch_add(local, Ordering::Relaxed);
        })
      })
      .collect();

    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for handle in handles {
      handle.join().unwrap();
    }

    let drained = drain_handle.join().unwrap_or(0);
    let total_duration = start.elapsed();
    let total_events = event_count.load(Ordering::Relaxed);
    let events_per_second = total_events as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "TTLog Real-Path Ingest Throughput".to_string(),
      metric: "send_event_fast Calls per Second".to_string(),
      value: events_per_second,
      unit: "events/sec".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: format!("threads={}, snapshot_buffer={}", thread_count, buffer_size),
      notes: format!("Produced={}, drained_broadcast={}", total_events, drained),
    }
  }

  /// Test maximum buffer operations per second
  fn max_buffer_operations(&self, buffer_size: usize) -> TestResult {
    let thread_count = 8; // Balanced for buffer operations
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size));
    let total_ops = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let end = Instant::now() + self.test_duration;

    let handles: Vec<_> = (0..thread_count)
      .map(|thread_id| {
        let buffer = Arc::clone(&buffer);
        let total_ops = Arc::clone(&total_ops);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);

        thread::spawn(move || {
          let mut local_ops = 0u64;
          let mut local_overwrites = 0u64;
          let mut counter = 0u64;
          barrier.wait();

          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            let event = create_minimal_event(thread_id as u64 * 1_000_000 + counter);
            counter += 1;

            // Count pushes and track overwrite pressure explicitly.
            match buffer.push(event) {
              Ok(Some(_)) => {
                local_ops += 1;
                local_overwrites += 1;
              },
              Ok(None) => {
                local_ops += 1;
              },
              Err(_) => {},
            }

            // Pop occasionally to maintain flow
            if counter.is_multiple_of(10) && buffer.pop().is_some() {
              local_ops += 1;
            }

            if (local_ops & 0x3FFF) == 0 {
              // flush every ~16k ops
              thread::yield_now();
            }
          }

          if local_overwrites > 0 {
            // Keep compiler from optimizing away overwrite accounting in hot loops.
            std::hint::black_box(local_overwrites);
          }
          total_ops.fetch_add(local_ops, Ordering::Relaxed);
        })
      })
      .collect();

    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for handle in handles {
      handle.join().unwrap();
    }

    let total_duration = start.elapsed();
    let total_operations = total_ops.load(Ordering::Relaxed);
    let ops_per_second = total_operations as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "Maximum Buffer Operations per Second".to_string(),
      metric: "Buffer Operations per Second".to_string(),
      value: ops_per_second,
      unit: "ops/sec".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: format!("threads={}, buffer={}", thread_count, buffer_size),
      notes: format!("Total ops: {}", total_operations),
    }
  }
}

// ============================================================================
// Concurrency Tester
// ============================================================================

struct ConcurrencyTester {
  test_duration: Duration,
}

impl ConcurrencyTester {
  fn new(test_duration: Duration) -> Self {
    Self { test_duration }
  }

  /// Test maximum concurrent threads
  fn max_concurrent_threads(&self, max_threads: usize) -> TestResult {
    let start = Instant::now();
    let mut successful_threads = 0;
    let mut max_ops_per_sec: f64 = 0.0;

    for thread_count in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024].iter() {
      if *thread_count > max_threads {
        break;
      }

      let test_start = Instant::now();
      let total_ops = Arc::new(AtomicU64::new(0));
      let barrier = Arc::new(Barrier::new(*thread_count + 1));
      let stop_flag = Arc::new(AtomicBool::new(false));
      let end = Instant::now() + self.test_duration;

      let handles: Vec<_> = (0..*thread_count)
        .map(|thread_id| {
          let total_ops = Arc::clone(&total_ops);
          let barrier = Arc::clone(&barrier);
          let stop_flag = Arc::clone(&stop_flag);

          thread::spawn(move || {
            let mut local_ops = 0u64;
            let mut counter = 0u64;
            barrier.wait();

            while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
              // Simulate work
              let hash = thread_id
                .wrapping_mul(counter as usize)
                .wrapping_add(0xdeadbeef);
              let _result = hash.rotate_left(7).wrapping_mul(0x9e3779b9);

              counter += 1;
              local_ops += 1;

              if (local_ops & 0x3FF) == 0 {
                thread::yield_now();
              }
            }

            total_ops.fetch_add(local_ops, Ordering::Relaxed);
          })
        })
        .collect();

      barrier.wait();
      let now = Instant::now();
      if end > now {
        thread::sleep(end - now);
      }
      stop_flag.store(true, Ordering::Release);

      let mut all_successful = true;
      for handle in handles {
        if handle.join().is_err() {
          all_successful = false;
          break;
        }
      }

      let test_duration = test_start.elapsed();
      let ops = total_ops.load(Ordering::Relaxed);
      let ops_per_sec = ops as f64 / test_duration.as_secs_f64();

      if all_successful {
        successful_threads = *thread_count;
        max_ops_per_sec = max_ops_per_sec.max(ops_per_sec);
      } else {
        break;
      }
    }

    let total_duration = start.elapsed();

    TestResult {
      test_name: "Maximum Concurrent Threads".to_string(),
      metric: "Maximum Threads".to_string(),
      value: successful_threads as f64,
      unit: "threads".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: format!("max_ops_per_sec={:.0}", max_ops_per_sec),
      notes: format!("Successfully ran {} concurrent threads", successful_threads),
    }
  }

  /// Test maximum provisioned buffers under a single-thread setup pass.
  fn max_concurrent_buffers(&self, max_buffers: usize) -> TestResult {
    let start = Instant::now();
    let mut successful_buffers = 0;
    let mut total_operations = 0u64;

    for buffer_count in [1, 10, 100, 1000, 5000].iter() {
      // Reduced max to avoid OOM
      if *buffer_count > max_buffers {
        break;
      }

      let test_start = Instant::now();
      let mut buffers = Vec::new();

      // Create buffers
      for _i in 0..*buffer_count {
        let buffer = LockFreeRingBuffer::<LogEvent>::new(1000);
        buffers.push(buffer);
      }

      // Test operations on all buffers
      let mut operations = 0u64;
      let mut all_successful = true;

      for (i, buffer) in buffers.iter().enumerate() {
        for j in 0..100 {
          let event = create_minimal_event((i * 100 + j) as u64);
          if buffer.push(event).is_ok() {
            operations += 1;
          } else {
            all_successful = false;
            break;
          }
        }
        if !all_successful {
          break;
        }
      }

      let test_duration = test_start.elapsed();

      if all_successful && test_duration < Duration::from_secs(30) {
        successful_buffers = *buffer_count;
        total_operations = operations;
      } else {
        break;
      }
    }

    let total_duration = start.elapsed();

    TestResult {
      test_name: "Maximum Provisioned Buffers".to_string(),
      metric: "Maximum Buffers (Single-thread setup)".to_string(),
      value: successful_buffers as f64,
      unit: "buffers".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: "ops_per_buffer=100".to_string(),
      notes: format!(
        "Single-thread provisioning smoke test, total operations: {}",
        total_operations
      ),
    }
  }
}

// ============================================================================
// Memory Efficiency Tester
// ============================================================================

struct MemoryEfficiencyTester {
  test_duration: Duration,
}

impl MemoryEfficiencyTester {
  fn new(test_duration: Duration) -> Self {
    Self { test_duration }
  }

  /// Run memory tests with multiple trials
  fn run_memory_tests(&self, runs: usize) -> (StatTestResult, StatTestResult, StatTestResult) {
    let alloc_results = run_multiple_times(
      || self.max_memory_allocation_rate(),
      runs,
      Duration::from_millis(100),
    );

    let efficiency_results = run_multiple_times(
      || self.bytes_per_event_efficiency(),
      runs,
      Duration::from_millis(100),
    );

    let throughput_results = run_multiple_times(
      || self.max_memory_throughput(),
      runs,
      Duration::from_millis(200),
    );

    (
      StatTestResult::from_results(&alloc_results),
      StatTestResult::from_results(&efficiency_results),
      StatTestResult::from_results(&throughput_results),
    )
  }

  /// Test memory allocation rate
  fn max_memory_allocation_rate(&self) -> TestResult {
    let start = Instant::now();
    let mut events = Vec::new();
    let mut allocation_count = 0u64;

    while start.elapsed() < self.test_duration {
      let event = create_minimal_event(allocation_count);
      events.push(event);
      allocation_count += 1;

      // Prevent excessive memory usage
      if events.len() > 100000 {
        events.clear();
      }
    }

    let duration = start.elapsed();
    let constructions_per_second = allocation_count as f64 / duration.as_secs_f64();

    TestResult {
      test_name: "Event Construction Rate".to_string(),
      metric: "Constructed Events per Second".to_string(),
      value: constructions_per_second,
      unit: "events/sec".to_string(),
      duration: format!("{:.3}s", duration.as_secs_f64()),
      config: format!("events={}", allocation_count),
      notes: format!(
        "Constructed LogEvent values in a Vec; this is not allocator call counting (est. footprint: {})",
        Self::format_bytes((allocation_count as f64 * Self::estimate_event_size() as f64) as usize)
      ),
    }
  }

  /// Test bytes per event efficiency
  fn bytes_per_event_efficiency(&self) -> TestResult {
    let start = Instant::now();
    let event_struct_bytes = std::mem::size_of::<LogEvent>() as f64;
    let sample_events: Vec<LogEvent> = (0..8).map(create_variable_size_event).collect();
    std::hint::black_box(&sample_events);
    let duration = start.elapsed();

    TestResult {
      test_name: "LogEvent Struct Size".to_string(),
      metric: "Static Struct Footprint".to_string(),
      value: event_struct_bytes,
      unit: "bytes/event".to_string(),
      duration: format!("{:.3}s", duration.as_secs_f64()),
      config: "type=LogEvent".to_string(),
      notes: "Excludes interned string payloads, allocator metadata, and listener/output buffers"
        .to_string(),
    }
  }

  /// Test peak memory throughput
  fn max_memory_throughput(&self) -> TestResult {
    let thread_count = 8;
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(100_000));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let end = Instant::now() + self.test_duration;

    let handles: Vec<_> = (0..thread_count)
      .map(|thread_id| {
        let buffer = Arc::clone(&buffer);
        let total_bytes = Arc::clone(&total_bytes);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);

        thread::spawn(move || {
          let mut local_bytes = 0u64;
          let mut counter = 0u64;
          let event_size = Self::estimate_event_size() as u64;
          barrier.wait();

          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            let event = create_minimal_event(thread_id as u64 * 1_000_000 + counter);
            if buffer.push(event).is_ok() {
              local_bytes += event_size;
              counter += 1;
            }

            if counter.is_multiple_of(100) {
              let _ = buffer.pop();
            }

            if (counter & 0x3FF) == 0 {
              thread::yield_now();
            }
          }

          total_bytes.fetch_add(local_bytes, Ordering::Relaxed);
        })
      })
      .collect();

    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for handle in handles {
      handle.join().unwrap();
    }

    let duration = start.elapsed();
    let total_bytes_processed = total_bytes.load(Ordering::Relaxed);
    let bytes_per_second = total_bytes_processed as f64 / duration.as_secs_f64();

    TestResult {
      test_name: "Memory Throughput".to_string(),
      metric: "Memory Processing Rate".to_string(),
      value: bytes_per_second,
      unit: "bytes/sec".to_string(),
      duration: format!("{:.3}s", duration.as_secs_f64()),
      config: format!("threads={}", thread_count),
      notes: format!(
        "Total: {}",
        Self::format_bytes(total_bytes_processed as usize)
      ),
    }
  }

  /// Estimate memory size of a standard event
  fn estimate_event_size() -> usize {
    std::mem::size_of::<LogEvent>()
  }

  /// Format bytes in human-readable format
  fn format_bytes(bytes: usize) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
      size /= 1024.0;
      unit_index += 1;
    }

    format!("{:.2} {}", size, UNITS[unit_index])
  }
}

// ============================================================================
// Buffer Operations Tester
// ============================================================================

struct BufferOperationsTester {
  test_duration: Duration,
}

impl BufferOperationsTester {
  fn new(test_duration: Duration) -> Self {
    Self { test_duration }
  }

  /// Run buffer tests with multiple trials and return statistical results
  fn run_buffer_tests_with_stats(&self, buffer_size: usize, runs: usize) -> Vec<StatBufferTest> {
    let configs = vec![(1, 1), (2, 2), (4, 4), (8, 8), (8, 4), (4, 8)];

    let mut stat_results = Vec::new();

    for (producers, consumers) in configs {
      let results = run_multiple_times(
        || self.test_buffer_operations(buffer_size, producers, consumers),
        runs,
        Duration::from_millis(150),
      );
      stat_results.push(StatBufferTest::from_results(&results));
    }

    stat_results
  }

  /// Test various producer/consumer ratios
  fn test_producer_consumer_ratios(&self, buffer_size: usize) -> Vec<BufferTest> {
    let configs = vec![
      (1, 1), // Balanced small
      (2, 2), // Balanced medium
      (4, 4), // Balanced optimal
      (8, 8), // Balanced high
      (8, 4), // Producer heavy
      (4, 8), // Consumer heavy
    ];

    let mut results = Vec::new();

    for (producers, consumers) in configs {
      let result = self.test_buffer_operations(buffer_size, producers, consumers);
      results.push(result);
    }

    results
  }

  fn test_buffer_operations(
    &self,
    buffer_size: usize,
    producers: usize,
    consumers: usize,
  ) -> BufferTest {
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size));
    let total_ops = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(producers + consumers + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let end = Instant::now() + self.test_duration;

    // Producer threads
    let producer_handles: Vec<_> = (0..producers)
      .map(|producer_id| {
        let buffer = Arc::clone(&buffer);
        let stop_flag = Arc::clone(&stop_flag);
        let total_ops = Arc::clone(&total_ops);
        let barrier = Arc::clone(&barrier);

        thread::spawn(move || {
          let mut local_ops = 0u64;
          let mut event_counter = 0u64;
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            let event = create_minimal_event(producer_id as u64 * 1000000 + event_counter);
            event_counter += 1;

            // Count pushes explicitly, including overwrite-mode success.
            if buffer.push(event).is_ok() {
              local_ops += 1
            }

            if local_ops.is_multiple_of(10000) {
              thread::yield_now();
            }
          }

          total_ops.fetch_add(local_ops, Ordering::Relaxed);
          local_ops
        })
      })
      .collect();

    // Consumer threads
    let consumer_handles: Vec<_> = (0..consumers)
      .map(|_consumer_id| {
        let buffer = Arc::clone(&buffer);
        let stop_flag = Arc::clone(&stop_flag);
        let total_ops = Arc::clone(&total_ops);
        let barrier = Arc::clone(&barrier);

        thread::spawn(move || {
          let mut local_ops = 0u64;
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            if buffer.pop().is_some() {
              local_ops += 1;
            } else {
              thread::yield_now();
            }

            if local_ops.is_multiple_of(10000) {
              thread::yield_now();
            }
          }

          total_ops.fetch_add(local_ops, Ordering::Relaxed);
          local_ops
        })
      })
      .collect();

    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for handle in producer_handles {
      handle.join().unwrap();
    }
    for handle in consumer_handles {
      handle.join().unwrap();
    }

    let duration = start.elapsed();
    let ops = total_ops.load(Ordering::Relaxed);
    let ops_per_sec = ops as f64 / duration.as_secs_f64();

    let notes = if producers > consumers {
      "Producer heavy".to_string()
    } else if consumers > producers {
      "Consumer heavy".to_string()
    } else {
      "Balanced".to_string()
    };

    BufferTest {
      test_name: format!("{}P/{}C", producers, consumers),
      producers,
      consumers,
      buffer_size,
      ops_per_sec,
      total_ops: ops,
      duration: format!("{:.3}s", duration.as_secs_f64()),
      notes,
    }
  }
}

// ============================================================================
// Statistics and Multi-Run Helpers
// ============================================================================

/// Calculate mean and standard deviation from a slice of values
fn mean_std(values: &[f64]) -> (f64, f64) {
  if values.is_empty() {
    return (0.0, 0.0);
  }
  let n = values.len() as f64;
  let mean = values.iter().sum::<f64>() / n;
  let variance = values.iter().map(|&x| (x - mean) * (x - mean)).sum::<f64>() / n.max(1.0);
  (mean, variance.sqrt())
}

/// Run a test function multiple times and return aggregated results
fn run_multiple_times<F, T>(test_fn: F, runs: usize, pause_between: Duration) -> Vec<T>
where
  F: Fn() -> T,
{
  let mut results = Vec::with_capacity(runs);
  for i in 0..runs {
    if i > 0 {
      thread::sleep(pause_between); // brief pause between runs
    }
    results.push(test_fn());
  }
  results
}

/// Enhanced TestResult with statistics
#[derive(Debug, Clone, Tabled)]
struct StatTestResult {
  #[tabled(rename = "Test Name")]
  test_name: String,
  #[tabled(rename = "Metric")]
  metric: String,
  #[tabled(rename = "Mean")]
  mean: f64,
  #[tabled(rename = "StdDev")]
  stddev: f64,
  #[tabled(rename = "Unit")]
  unit: String,
  #[tabled(rename = "Runs")]
  runs: usize,
  #[tabled(rename = "Config")]
  config: String,
  #[tabled(rename = "Notes")]
  notes: String,
}

impl StatTestResult {
  fn from_results(results: &[TestResult]) -> Self {
    if results.is_empty() {
      return Self {
        test_name: "Empty".to_string(),
        metric: "N/A".to_string(),
        mean: 0.0,
        stddev: 0.0,
        unit: "N/A".to_string(),
        runs: 0,
        config: "N/A".to_string(),
        notes: "No results".to_string(),
      };
    }

    let values: Vec<f64> = results.iter().map(|r| r.value).collect();
    let (mean, stddev) = mean_std(&values);
    let first = &results[0];

    Self {
      test_name: first.test_name.clone(),
      metric: first.metric.clone(),
      mean,
      stddev,
      unit: first.unit.clone(),
      runs: results.len(),
      config: first.config.clone(),
      notes: format!(
        "Range: {:.1} - {:.1}",
        values.iter().fold(f64::INFINITY, |a, &b| a.min(b)),
        values.iter().fold(f64::NEG_INFINITY, |a, &b| a.max(b))
      ),
    }
  }
}

// ============================================================================
// End-to-End Logging Pipeline Benchmark
// ============================================================================

#[derive(Clone, Debug)]
enum SinkKind {
  Null,
  File(PathBuf),
}

#[derive(Clone, Debug)]
struct EndToEndConfig {
  name: &'static str,
  duration: Duration,
  producers: usize,
  consumers: usize,
  buffer_size: usize,
  target_produced_per_sec: Option<u64>,
  sink: SinkKind,
}

#[derive(Debug, Clone, Tabled)]
struct EndToEndResult {
  #[tabled(rename = "Test")]
  test_name: String,
  #[tabled(rename = "Producers")]
  producers: usize,
  #[tabled(rename = "Consumers")]
  consumers: usize,
  #[tabled(rename = "Buffer Size")]
  buffer_size: usize,
  #[tabled(rename = "Produced/s")]
  produced_per_sec: f64,
  #[tabled(rename = "Consumed/s")]
  consumed_per_sec: f64,
  #[tabled(rename = "Drops")]
  drops: u64,
  #[tabled(rename = "Drop %")]
  drop_rate_percent: String,
  #[tabled(rename = "Bytes Written")]
  bytes_written: u64,
  #[tabled(rename = "p50 (us)")]
  p50_us: f64,
  #[tabled(rename = "p95 (us)")]
  p95_us: f64,
  #[tabled(rename = "p99 (us)")]
  p99_us: f64,
  #[tabled(rename = "p999 (us)")]
  p999_us: f64,
}

struct EndToEndTester {
  config: EndToEndConfig,
}

impl EndToEndTester {
  fn new(config: EndToEndConfig) -> Self {
    Self { config }
  }

  fn run(&self) -> EndToEndResult {
    let cfg = &self.config;
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(cfg.buffer_size));
    let barrier = Arc::new(Barrier::new(cfg.producers + cfg.consumers + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let end = Instant::now() + cfg.duration;
    let t0 = Instant::now();

    // For metrics
    let produced = Arc::new(AtomicU64::new(0));
    let consumed = Arc::new(AtomicU64::new(0));
    let drops = Arc::new(AtomicU64::new(0));
    let bytes_written = Arc::new(AtomicU64::new(0));

    // Latencies captured as microseconds; per-consumer vectors merged after join
    let latencies: Arc<Mutex<Vec<u64>>> = Arc::new(Mutex::new(Vec::new()));

    // Build sink if needed
    let writer: Option<Arc<Mutex<BufWriter<File>>>> = match &cfg.sink {
      SinkKind::Null => None,
      SinkKind::File(path) => {
        let file = File::create(path)
          .unwrap_or_else(|e| panic!("failed to create E2E output file {:?}: {}", path, e));
        Some(Arc::new(Mutex::new(BufWriter::new(file))))
      },
    };
    let producers_count = cfg.producers;
    let target_produced_per_sec = cfg.target_produced_per_sec;

    // Producers
    let producer_handles: Vec<_> = (0..cfg.producers)
      .map(|pid| {
        let buffer = Arc::clone(&buffer);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);
        let produced = Arc::clone(&produced);
        let drops = Arc::clone(&drops);
        thread::spawn(move || {
          let target_interval_ns = target_produced_per_sec.map(|rate| {
            let per_thread_rate = (rate / producers_count.max(1) as u64).max(1);
            (1_000_000_000u64 / per_thread_rate).max(1)
          });
          let mut next_emit_at = Instant::now();
          let mut local_prod = 0u64;
          let mut local_drops = 0u64;
          let mut ctr = 0u64;
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            if let Some(interval_ns) = target_interval_ns {
              let now = Instant::now();
              if now < next_emit_at {
                std::hint::spin_loop();
                continue;
              }
              next_emit_at = now
                .checked_add(Duration::from_nanos(interval_ns))
                .unwrap_or(now);
            }

            // enqueue timestamp (us since t0) into position.0 for latency measurement
            let ts_us = (Instant::now().duration_since(t0).as_micros() as u64) as u32;
            let mut ev = create_minimal_event(pid as u64 * 1_000_000 + ctr);
            ev.position = (ts_us, 0);
            match buffer.push(ev) {
              Ok(Some(_evicted)) => {
                local_prod += 1;
                local_drops += 1;
              },
              Ok(None) => {
                local_prod += 1;
              },
              Err(_) => {
                local_drops += 1;
              },
            }
            ctr += 1;
            if (ctr & 0x3FF) == 0 {
              thread::yield_now();
            }
          }
          produced.fetch_add(local_prod, Ordering::Relaxed);
          drops.fetch_add(local_drops, Ordering::Relaxed);
        })
      })
      .collect();

    // Consumers
    let consumer_handles: Vec<_> = (0..cfg.consumers)
      .map(|_cid| {
        let buffer = Arc::clone(&buffer);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);
        let consumed = Arc::clone(&consumed);
        let latencies = Arc::clone(&latencies);
        let bytes_written = Arc::clone(&bytes_written);
        let writer = writer.clone();
        thread::spawn(move || {
          const WRITE_BATCH_BYTES: usize = 64 * 1024;
          const WRITE_BATCH_EVENTS: u64 = 256;
          let mut local_cons = 0u64;
          let mut local_lat: Vec<u64> = Vec::with_capacity(64_000);
          let mut local_bytes = 0u64;
          let mut write_batch = String::with_capacity(WRITE_BATCH_BYTES);
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            if let Some(ev) = buffer.pop() {
              // latency: now - enqueue
              let now_us = Instant::now().duration_since(t0).as_micros() as u64;
              let enq_us = ev.position.0 as u64;
              let lat = now_us.saturating_sub(enq_us);
              local_lat.push(lat);

              // format and optionally write
              if let Some(w) = &writer {
                // Batch lines locally to reduce lock/write frequency.
                let _ = writeln!(
                  &mut write_batch,
                  "{},{}:{}",
                  ev.target_id, ev.file_id, ev.position.0
                );

                if write_batch.len() >= WRITE_BATCH_BYTES
                  || local_cons.is_multiple_of(WRITE_BATCH_EVENTS)
                {
                  let mut w = w.lock().unwrap();
                  if w.write_all(write_batch.as_bytes()).is_ok() {
                    local_bytes += write_batch.len() as u64;
                    write_batch.clear();
                  }
                }
              }

              local_cons += 1;
            } else {
              thread::yield_now();
            }
            if (local_cons & 0x3FF) == 0 {
              thread::yield_now();
            }
          }
          // flush any writer
          if let Some(w) = &writer {
            let mut w = w.lock().unwrap();
            if !write_batch.is_empty() {
              if w.write_all(write_batch.as_bytes()).is_ok() {
                local_bytes += write_batch.len() as u64;
              }
              write_batch.clear();
            }
            let _ = w.flush();
          }
          consumed.fetch_add(local_cons, Ordering::Relaxed);
          bytes_written.fetch_add(local_bytes, Ordering::Relaxed);
          // merge latencies
          let mut g = latencies.lock().unwrap();
          g.extend(local_lat);
        })
      })
      .collect();

    // Start window
    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for h in producer_handles {
      let _ = h.join();
    }
    for h in consumer_handles {
      let _ = h.join();
    }

    let dur = start.elapsed();
    let prod = produced.load(Ordering::Relaxed);
    let cons = consumed.load(Ordering::Relaxed);
    let drp = drops.load(Ordering::Relaxed);
    let bytes = bytes_written.load(Ordering::Relaxed);

    // percentiles
    let mut l = latencies.lock().unwrap();
    l.sort_unstable();
    let p = |q: f64| -> f64 {
      if l.is_empty() {
        return 0.0;
      }
      let idx = ((q * (l.len() as f64 - 1.0)).round() as usize).min(l.len() - 1);
      l[idx] as f64
    };

    EndToEndResult {
      test_name: cfg.name.to_string(),
      producers: cfg.producers,
      consumers: cfg.consumers,
      buffer_size: cfg.buffer_size,
      produced_per_sec: prod as f64 / dur.as_secs_f64(),
      consumed_per_sec: cons as f64 / dur.as_secs_f64(),
      drops: drp,
      drop_rate_percent: if prod > 0 {
        format!("{:.2}", (drp as f64 * 100.0) / prod as f64)
      } else {
        "0.00".to_string()
      },
      bytes_written: bytes,
      p50_us: p(0.50),
      p95_us: p(0.95),
      p99_us: p(0.99),
      p999_us: p(0.999),
    }
  }

  fn run_with_stats(&self, runs: usize) -> Vec<EndToEndResult> {
    run_multiple_times(|| self.run(), runs, Duration::from_millis(200))
  }
}

// ============================================================================
// Utility Functions
// ============================================================================

fn create_minimal_event(counter: u64) -> LogEvent {
  static INTERNER: std::sync::OnceLock<Arc<StringInterner>> = std::sync::OnceLock::new();
  let interner = INTERNER.get_or_init(|| Arc::new(StringInterner::new()));

  // Intern common strings
  let target_id = interner.intern_target("bench");
  let message_id = std::num::NonZeroU16::new(interner.intern_message("test message"));
  let kv_id = None;
  let file_id = interner.intern_file(file!());

  // Pack metadata: timestamp (ms), level, thread_id (0 for benchmark)
  let ts_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_else(|_| std::time::Duration::from_millis(0))
    .as_millis() as u64;
  let packed = LogEvent::pack_meta(ts_ms, LogLevel::INFO, 0);

  let mut ev = LogEvent {
    packed_meta: packed,
    target_id,
    kv_id,
    message_id,
    file_id,
    position: (0, 0),
  };

  // Optionally use counter to vary position for uniqueness
  let line = (counter & 0xFFFF) as u32;
  ev.position = (line, 0);
  ev
}

fn create_variable_size_event(counter: u64) -> LogEvent {
  static INTERNER: std::sync::OnceLock<Arc<StringInterner>> = std::sync::OnceLock::new();
  let interner = INTERNER.get_or_init(|| Arc::new(StringInterner::new()));

  let message = match counter % 3 {
    0 => "short",
    1 => "medium length message with more content",
    _ => "very long message with extensive content that takes up significantly more memory",
  };

  let target_id = interner.intern_target("bench");
  let message_id = std::num::NonZeroU16::new(interner.intern_message(message));
  let file_id = interner.intern_file(file!());
  let kv_id = None;

  let ts_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_else(|_| std::time::Duration::from_millis(0))
    .as_millis() as u64;
  let packed = LogEvent::pack_meta(ts_ms, LogLevel::INFO, 0);

  let mut ev = LogEvent {
    packed_meta: packed,
    target_id,
    kv_id,
    message_id,
    file_id,
    position: (0, 0),
  };

  let line = (counter & 0xFFFF) as u32;
  ev.position = (line, 0);
  ev
}

// ============================================================================
// Comprehensive Benchmark Runner with Statistics
// ============================================================================

struct ComprehensiveBenchmark {
  test_duration: Duration,
  runs_per_test: usize,
}

impl ComprehensiveBenchmark {
  fn new(test_duration: Duration, runs_per_test: usize) -> Self {
    Self {
      test_duration,
      runs_per_test,
    }
  }

  /// Run all benchmarks with statistical analysis
  fn run_statistical_benchmarks(&self) -> BenchmarkSuite {
    println!("🚀 Starting Comprehensive Performance Benchmark Suite");
    println!(
      "📊 Running {} trials per test for statistical confidence",
      self.runs_per_test
    );
    println!(
      "⏱️  Test duration: {:.1}s per trial\n",
      self.test_duration.as_secs_f64()
    );

    let buffer_sizes = vec![1024, 8192, 65536]; // Realistic buffer sizes
    let mut suite = BenchmarkSuite::new();

    // Throughput tests
    println!("🔥 Running Throughput Tests...");
    let throughput_tester = ThroughputTester::new(self.test_duration);
    for &buffer_size in &buffer_sizes {
      let (events_stat, buffer_stat) =
        throughput_tester.run_throughput_tests(buffer_size, self.runs_per_test);
      suite.throughput_results.push(events_stat);
      suite.throughput_results.push(buffer_stat);
    }

    // Memory tests
    println!("💾 Running Memory Tests...");
    let memory_tester = MemoryEfficiencyTester::new(self.test_duration);
    let (alloc_stat, efficiency_stat, throughput_stat) =
      memory_tester.run_memory_tests(self.runs_per_test);
    suite.memory_results.push(alloc_stat);
    suite.memory_results.push(efficiency_stat);
    suite.memory_results.push(throughput_stat);

    // Buffer operation tests
    println!("🔄 Running Buffer Operation Tests...");
    let buffer_tester = BufferOperationsTester::new(self.test_duration);
    for &buffer_size in &buffer_sizes {
      let buffer_stats = buffer_tester.run_buffer_tests_with_stats(buffer_size, self.runs_per_test);
      suite.buffer_results.extend(buffer_stats);
    }

    // Concurrency tests (single run, as they test scaling)
    println!("⚡ Running Concurrency Tests...");
    let concurrency_tester = ConcurrencyTester::new(Duration::from_millis(500));
    let thread_result = concurrency_tester.max_concurrent_threads(256);
    let buffer_result = concurrency_tester.max_concurrent_buffers(1000); // Reduced from 100k
    suite.concurrency_results.push(thread_result);
    suite.concurrency_results.push(buffer_result);

    println!("✅ Benchmark suite completed!\n");
    suite
  }

  /// Legacy method for backward compatibility
  #[allow(dead_code)]
  fn run_all_benchmarks(&self) -> (Vec<TestResult>, Vec<BufferTest>, Vec<SummaryMetric>) {
    let mut all_test_results = Vec::new();
    let mut all_buffer_results = Vec::new();
    let mut summary_metrics = Vec::new();

    println!("🚀 Starting Comprehensive TTLog Performance Analysis...");
    println!("{}", "=".repeat(80));

    // ========================================
    // Throughput Tests
    // ========================================
    println!("\n📊 Running Throughput Tests...");
    let throughput_tester = ThroughputTester::new(self.test_duration);

    let max_events_result = throughput_tester.max_events_per_second(1_000_000);
    println!(
      "✅ Max Events/Sec: {:.0} events/sec",
      max_events_result.value
    );

    let max_ops_result = throughput_tester.max_buffer_operations(1_000_000);
    println!("✅ Max Buffer Ops/Sec: {:.0} ops/sec", max_ops_result.value);

    all_test_results.push(max_events_result.clone());
    all_test_results.push(max_ops_result.clone());

    summary_metrics.extend_from_slice(&[
      SummaryMetric {
        metric: "Maximum Events per Second".to_string(),
        value: max_events_result.value,
        unit: max_events_result.unit.clone(),
      },
      SummaryMetric {
        metric: "Maximum Buffer Operations per Second".to_string(),
        value: max_ops_result.value,
        unit: max_ops_result.unit.clone(),
      },
    ]);

    // Display throughput results table
    println!("\n📋 Throughput Test Results:");
    let throughput_table =
      Table::new(&[max_events_result.clone(), max_ops_result.clone()]).to_string();
    println!("{}", throughput_table);

    // ========================================
    // Concurrency Tests
    // ========================================
    println!("\n📊 Running Concurrency Tests...");
    let concurrency_tester = ConcurrencyTester::new(Duration::from_secs(10));

    let max_threads_result = concurrency_tester.max_concurrent_threads(1024);
    println!(
      "✅ Max Concurrent Threads: {:.0} threads",
      max_threads_result.value
    );

    let max_buffers_result = concurrency_tester.max_concurrent_buffers(100_000);
    println!(
      "✅ Max Concurrent Buffers: {:.0} buffers",
      max_buffers_result.value
    );

    all_test_results.push(max_threads_result.clone());
    all_test_results.push(max_buffers_result.clone());

    summary_metrics.extend_from_slice(&[
      SummaryMetric {
        metric: "Maximum Concurrent Threads".to_string(),
        value: max_threads_result.value,
        unit: max_threads_result.unit.clone(),
      },
      SummaryMetric {
        metric: "Maximum Concurrent Buffers".to_string(),
        value: max_buffers_result.value,
        unit: max_buffers_result.unit.clone(),
      },
    ]);

    // Display concurrency results table
    println!("\n📋 Concurrency Test Results:");
    let concurrency_table =
      Table::new(&[max_threads_result.clone(), max_buffers_result.clone()]).to_string();
    println!("{}", concurrency_table);

    // ========================================
    // Memory Tests
    // ========================================
    println!("\n📊 Running Memory Tests...");
    let memory_tester = MemoryEfficiencyTester::new(self.test_duration);

    let memory_allocation_result = memory_tester.max_memory_allocation_rate();
    println!(
      "✅ Memory Allocation Rate: {:.0} allocs/sec",
      memory_allocation_result.value
    );

    let bytes_per_event_result = memory_tester.bytes_per_event_efficiency();
    println!(
      "✅ Bytes per Event (approx): {:.2} bytes/event",
      bytes_per_event_result.value
    );

    let memory_throughput_result = memory_tester.max_memory_throughput();
    println!(
      "✅ Memory Throughput: {:.0} bytes/sec",
      memory_throughput_result.value
    );

    all_test_results.push(memory_allocation_result.clone());
    all_test_results.push(bytes_per_event_result.clone());
    all_test_results.push(memory_throughput_result.clone());

    summary_metrics.extend_from_slice(&[
      SummaryMetric {
        metric: "Constructed Events per Second".to_string(),
        value: memory_allocation_result.value,
        unit: memory_allocation_result.unit.clone(),
      },
      SummaryMetric {
        metric: "LogEvent Struct Size".to_string(),
        value: bytes_per_event_result.value,
        unit: bytes_per_event_result.unit.clone(),
      },
      SummaryMetric {
        metric: "Memory Throughput".to_string(),
        value: memory_throughput_result.value,
        unit: memory_throughput_result.unit.clone(),
      },
    ]);

    // ========================================
    // Buffer Operations Tests (various P/C ratios)
    // ========================================
    println!("\n📊 Running Buffer Operations Tests...");
    let buffer_tester = BufferOperationsTester::new(self.test_duration);
    let ratios_results = buffer_tester.test_producer_consumer_ratios(1_000_000);
    all_buffer_results.extend_from_slice(&ratios_results);

    // Return all collected results
    (all_test_results, all_buffer_results, summary_metrics)
  }
}

// =========================================================================
// Unification and Printing
// =========================================================================

#[allow(dead_code)]
fn buffer_tests_to_results(buffer_tests: &[BufferTest]) -> Vec<TestResult> {
  buffer_tests
    .iter()
    .map(|b| TestResult {
      test_name: format!("Buffer {}", b.test_name),
      metric: "Ops per Second".to_string(),
      value: b.ops_per_sec,
      unit: "ops/sec".to_string(),
      duration: b.duration.clone(),
      config: format!(
        "producers={}, consumers={}, buffer={}",
        b.producers, b.consumers, b.buffer_size
      ),
      notes: format!("total_ops={}, {}", b.total_ops, b.notes),
    })
    .collect()
}

#[allow(dead_code)]
fn print_unified_results_table(results: &[TestResult]) {
  println!("\n{}", "=".repeat(100));
  println!("🚀 ALL PERFORMANCE RESULTS (Unified Table)");
  println!("{}", "=".repeat(100));
  if results.is_empty() {
    println!("No results to display");
    return;
  }
  let table = Table::new(results).to_string();
  println!("{}", table);
  println!("{}", "=".repeat(100));
}

// =========================================================================
// Main
// =========================================================================

/// Complete benchmark suite results
#[derive(Debug)]
struct BenchmarkSuite {
  throughput_results: Vec<StatTestResult>,
  memory_results: Vec<StatTestResult>,
  buffer_results: Vec<StatBufferTest>,
  concurrency_results: Vec<TestResult>,
}

impl BenchmarkSuite {
  fn new() -> Self {
    Self {
      throughput_results: Vec::new(),
      memory_results: Vec::new(),
      buffer_results: Vec::new(),
      concurrency_results: Vec::new(),
    }
  }

  fn display_results(&self) {
    println!("\n🎯 === STATISTICAL PERFORMANCE BENCHMARK RESULTS ===");

    if !self.throughput_results.is_empty() {
      println!("\n🔥 Throughput Tests (Mean ± StdDev):");
      let table = Table::new(&self.throughput_results).to_string();
      println!("{}", table);
    }

    if !self.memory_results.is_empty() {
      println!("\n💾 Memory Tests (Mean ± StdDev):");
      let table = Table::new(&self.memory_results).to_string();
      println!("{}", table);
    }

    if !self.buffer_results.is_empty() {
      println!("\n🔄 Buffer Operation Tests (Mean ± StdDev):");
      let table = Table::new(&self.buffer_results).to_string();
      println!("{}", table);
    }

    if !self.concurrency_results.is_empty() {
      println!("\n⚡ Concurrency Tests:");
      let table = Table::new(&self.concurrency_results).to_string();
      println!("{}", table);
    }

    println!("\n📋 Benchmark Notes:");
    println!("• Throughput/memory tests run with multiple trials and report mean/stddev/range");
    println!("• Throughput section now uses ttlog `send_event_fast` real ingest path");
    println!(
      "• Buffer and memory metrics are labeled as synthetic where they are not full pipeline"
    );
    println!("• End-to-end results include drop counting that treats overwrite as data loss");
    println!("• File sink benchmark now fails fast if output file cannot be created");
    println!("\n✅ Statistical benchmark suite completed successfully!");
  }
}

#[derive(Debug, Clone)]
struct BenchmarkContext {
  run_utc: String,
  os: String,
  arch: String,
  logical_cpus: usize,
  available_parallelism: usize,
  build_profile: String,
  debug_assertions_enabled: bool,
  test_duration_secs: f64,
  runs_per_test: usize,
}

impl BenchmarkContext {
  fn collect(test_duration: Duration, runs_per_test: usize) -> Self {
    let logical_cpus = std::thread::available_parallelism()
      .map(|n| n.get())
      .unwrap_or(1);
    let build_profile = std::env::current_exe()
      .ok()
      .map(|p| p.to_string_lossy().to_string())
      .map(|p| {
        if p.contains("/target/release/") {
          "release".to_string()
        } else if p.contains("/target/debug/") {
          "debug".to_string()
        } else {
          "unknown".to_string()
        }
      })
      .unwrap_or_else(|| "unknown".to_string());
    Self {
      run_utc: Utc::now().to_rfc3339(),
      os: std::env::consts::OS.to_string(),
      arch: std::env::consts::ARCH.to_string(),
      logical_cpus,
      available_parallelism: logical_cpus,
      build_profile,
      debug_assertions_enabled: cfg!(debug_assertions),
      test_duration_secs: test_duration.as_secs_f64(),
      runs_per_test,
    }
  }
}

fn table_block<T: Tabled>(rows: &[T]) -> String {
  if rows.is_empty() {
    return "_No rows_".to_string();
  }
  format!("```\n{}\n```", Table::new(rows))
}

fn write_audit_report(
  context: &BenchmarkContext,
  suite: &BenchmarkSuite,
  e2e_null_results: &[EndToEndResult],
  e2e_file_results: &[EndToEndResult],
) -> std::io::Result<PathBuf> {
  fs::create_dir_all("ttlog-benches/reports")?;
  let output_path = PathBuf::from("ttlog-benches/reports/latest.md");

  let mut report = String::new();
  report.push_str("# TTLog Benchmark Audit Report\n\n");
  report.push_str("## Run Metadata\n");
  report.push_str(&format!("- UTC: {}\n", context.run_utc));
  report.push_str(&format!("- OS/Arch: {}/{}\n", context.os, context.arch));
  report.push_str(&format!("- Logical CPUs: {}\n", context.logical_cpus));
  report.push_str(&format!(
    "- Available Parallelism: {}\n",
    context.available_parallelism
  ));
  report.push_str(&format!("- Build Profile: {}\n", context.build_profile));
  report.push_str(&format!(
    "- Debug Assertions Enabled: {}\n",
    context.debug_assertions_enabled
  ));
  report.push_str(&format!(
    "- Per-trial Duration: {:.3}s\n",
    context.test_duration_secs
  ));
  report.push_str(&format!("- Runs per Test: {}\n\n", context.runs_per_test));

  report.push_str("## Integrity Policy\n");
  report.push_str("- Throughput numbers must identify whether they are real-path or synthetic.\n");
  report.push_str("- Drop counting must include overwrite-driven loss.\n");
  report.push_str("- File sink benchmarks must fail if the file cannot be created.\n");
  report.push_str("- Memory metrics must state what is included/excluded.\n\n");

  report.push_str("## Throughput (Statistical)\n");
  report.push_str(&table_block(&suite.throughput_results));
  report.push_str("\n\n## Memory (Statistical)\n");
  report.push_str(&table_block(&suite.memory_results));
  report.push_str("\n\n## Buffer Ops (Statistical)\n");
  report.push_str(&table_block(&suite.buffer_results));
  report.push_str("\n\n## Concurrency\n");
  report.push_str(&table_block(&suite.concurrency_results));
  report.push_str("\n\n## End-to-End Null Sink\n");
  report.push_str(&table_block(e2e_null_results));
  report.push_str("\n\n## End-to-End File Sink\n");
  report.push_str(&table_block(e2e_file_results));
  report.push('\n');

  fs::write(&output_path, report)?;
  Ok(output_path)
}

fn main() {
  let test_duration_secs = std::env::var("TTLOG_BENCH_DURATION_SECS")
    .ok()
    .and_then(|v| v.parse::<u64>().ok())
    .unwrap_or(3);
  let runs_per_test = std::env::var("TTLOG_BENCH_RUNS")
    .ok()
    .and_then(|v| v.parse::<usize>().ok())
    .unwrap_or(5);
  let test_duration = Duration::from_secs(test_duration_secs);
  let context = BenchmarkContext::collect(test_duration, runs_per_test);
  let benchmark = ComprehensiveBenchmark::new(test_duration, runs_per_test);

  let suite = benchmark.run_statistical_benchmarks();
  suite.display_results();

  // =============================================================
  // End-to-End pipeline benchmarks (through ttlog-like path)
  // =============================================================
  println!("\n🧪 End-to-End Pipeline Benchmarks:");
  let e2e_cfg_null = EndToEndConfig {
    name: "E2E-Null",
    duration: test_duration,
    producers: 8,
    consumers: 1,
    buffer_size: 65_536,
    target_produced_per_sec: None,
    sink: SinkKind::Null,
  };
  let e2e_null = EndToEndTester::new(e2e_cfg_null);
  let e2e_null_results = e2e_null.run_with_stats(runs_per_test);
  println!("\n• E2E (Null sink):");
  let table = Table::new(&e2e_null_results).to_string();
  println!("{}", table);

  // File sink sustained profile: tuned for low-loss operation.
  let e2e_cfg_file_sustained = EndToEndConfig {
    name: "E2E-File-Sustained",
    duration: test_duration,
    producers: 4,
    consumers: 1,
    buffer_size: 1_048_576,
    target_produced_per_sec: Some(3_000_000),
    sink: SinkKind::File(PathBuf::from("/tmp/ttlog_e2e_bench.log")),
  };
  let e2e_file_sustained = EndToEndTester::new(e2e_cfg_file_sustained);
  let e2e_file_sustained_results = e2e_file_sustained.run_with_stats(runs_per_test);
  println!("\n• E2E (File sink, sustained profile):");
  let table = Table::new(&e2e_file_sustained_results).to_string();
  println!("{}", table);

  // File sink stress profile: overload behavior and drop characteristics.
  let e2e_cfg_file_stress = EndToEndConfig {
    name: "E2E-File-Stress",
    duration: test_duration,
    producers: 8,
    consumers: 1,
    buffer_size: 65_536,
    target_produced_per_sec: None,
    sink: SinkKind::File(PathBuf::from("/tmp/ttlog_e2e_bench.log")),
  };
  let e2e_file_stress = EndToEndTester::new(e2e_cfg_file_stress);
  let e2e_file_stress_results = e2e_file_stress.run_with_stats(runs_per_test);
  println!("\n• E2E (File sink, stress profile):");
  let table = Table::new(&e2e_file_stress_results).to_string();
  println!("{}", table);

  let mut e2e_file_results = Vec::new();
  e2e_file_results.extend(e2e_file_sustained_results);
  e2e_file_results.extend(e2e_file_stress_results);
  println!("\n• E2E (File sink, combined):");
  let table = Table::new(&e2e_file_results).to_string();
  println!("{}", table);

  match write_audit_report(&context, &suite, &e2e_null_results, &e2e_file_results) {
    Ok(path) => println!("\n📝 Wrote audit report: {}", path.display()),
    Err(e) => eprintln!("\n[warn] failed to write audit report: {}", e),
  }
}
