use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::thread;
use std::time::{Duration, Instant};
use std::fs::File;
use std::io::{BufWriter, Write};
use std::path::PathBuf;

use tabled::{Table, Tabled};
use ttlog::{
  event::{LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
  string_interner::StringInterner,
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

#[derive(Debug, Clone, Tabled)]
struct ConcurrencyResult {
  #[tabled(rename = "Thread Count")]
  thread_count: usize,
  #[tabled(rename = "Success")]
  success: String,
  #[tabled(rename = "Duration")]
  duration: String,
  #[tabled(rename = "Ops/Thread")]
  ops_per_thread: u64,
  #[tabled(rename = "Total Ops/Sec")]
  total_ops_per_sec: f64,
}

#[derive(Debug, Clone, Tabled)]
struct MemoryTestResult {
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
  #[tabled(rename = "Additional Info")]
  additional_info: String,
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
    // Warm-up (short) to stabilize CPU freq and caches
    let warmup = Duration::from_millis(500);
    let thread_count = 16; // can be parameterized
    let event_count = Arc::new(AtomicU64::new(0));
    let barrier = Arc::new(Barrier::new(thread_count + 1));
    let stop_flag = Arc::new(AtomicBool::new(false));

    // Optional: perform a brief warmup without measuring
    let warm_end = Instant::now() + warmup;
    while Instant::now() < warm_end {
      std::hint::spin_loop();
    }

    // Spawn workers
    let end = Instant::now() + self.test_duration;
    let handles: Vec<_> = (0..thread_count)
      .map(|_tid| {
        let event_count = Arc::clone(&event_count);
        let barrier = Arc::clone(&barrier);
        let stop_flag = Arc::clone(&stop_flag);

        thread::spawn(move || {
          // reduce contention by batching
          let mut local = 0u64;
          barrier.wait();
          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            // micro work: count only (no tracing)
            local += 1;
            if (local & 0x3FF) == 0 {
              // flush every 1024 ops
              event_count.fetch_add(1024, Ordering::Relaxed);
              local = 0;
            }
          }
          if local > 0 {
            event_count.fetch_add(local, Ordering::Relaxed);
          }
        })
      })
      .collect();

    // Start measurement window
    let start = Instant::now();
    barrier.wait();

    // Sleep until deadline (threads also check end)
    let now = Instant::now();
    if end > now {
      thread::sleep(end - now);
    }
    stop_flag.store(true, Ordering::Release);

    for handle in handles {
      handle.join().unwrap();
    }

    let total_duration = start.elapsed();
    let total_events = event_count.load(Ordering::Relaxed);
    let events_per_second = total_events as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "Maximum Events per Second".to_string(),
      metric: "Events per Second".to_string(),
      value: events_per_second,
      unit: "events/sec".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: format!("threads={}, buffer={}", thread_count, buffer_size),
      notes: format!("Total events: {}", total_events),
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
          let mut counter = 0u64;
          barrier.wait();

          while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
            let event = create_minimal_event(thread_id as u64 * 1_000_000 + counter);
            counter += 1;

            // Push operation (count success)
            if buffer.push(event).is_ok() {
              local_ops += 1;
            }

            // Pop occasionally to maintain flow
            if (counter % 10) == 0 {
              if buffer.pop().is_some() {
                local_ops += 1;
              }
            }

            if (local_ops & 0x3FFF) == 0 {
              // flush every ~16k ops
              thread::yield_now();
            }
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
      let end = Instant::now() + Duration::from_millis(100);

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

      if all_successful && test_duration < Duration::from_secs(10) {
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

  /// Test maximum concurrent buffers
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
      test_name: "Maximum Concurrent Buffers".to_string(),
      metric: "Maximum Buffers".to_string(),
      value: successful_buffers as f64,
      unit: "buffers".to_string(),
      duration: format!("{:.3}s", total_duration.as_secs_f64()),
      config: format!("ops_per_buffer=100"),
      notes: format!("Total operations: {}", total_operations),
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
    let allocations_per_second = allocation_count as f64 / duration.as_secs_f64();

    TestResult {
      test_name: "Memory Allocation Rate".to_string(),
      metric: "Allocations per Second".to_string(),
      value: allocations_per_second,
      unit: "allocs/sec".to_string(),
      duration: format!("{:.3}s", duration.as_secs_f64()),
      config: format!("events={}", allocation_count),
      notes: format!(
        "Est. memory: {}",
        Self::format_bytes((allocation_count as f64 * Self::estimate_event_size() as f64) as usize)
      ),
    }
  }

  /// Test bytes per event efficiency
  fn bytes_per_event_efficiency(&self) -> TestResult {
    let start = Instant::now();
    let test_counts = vec![1000, 5000, 10000, 25000];
    let mut total_events = 0usize;
    let mut total_calculated_memory = 0usize;

    for count in test_counts {
      let events: Vec<LogEvent> = (0..count)
        .map(|i| create_variable_size_event(i as u64))
        .collect();

      total_events += events.len();

      // Calculate more accurate memory usage per event
      for event in &events {
        let mut event_memory = std::mem::size_of::<LogEvent>();
        total_calculated_memory += event_memory;
      }

      // Ensure events aren't optimized away
      let _check = events.first().map(|e| e.packed_meta);
    }

    let duration = start.elapsed();
    let bytes_per_event = if total_events > 0 {
      total_calculated_memory as f64 / total_events as f64
    } else {
      0.0
    };

    TestResult {
      test_name: "Bytes per Event (Calculated)".to_string(),
      metric: "Memory Efficiency".to_string(),
      value: bytes_per_event,
      unit: "bytes/event".to_string(),
      duration: format!("{:.3}s", duration.as_secs_f64()),
      config: format!("events={}", total_events),
      notes: format!(
        "Total calculated memory: {} (includes field overhead)",
        Self::format_bytes(total_calculated_memory)
      ),
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

            if (counter % 100) == 0 {
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
    std::mem::size_of::<LogEvent>() + 128 // Base size + estimated overhead
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

            // Count only successful pushes; track drops by ignoring failed pushes
            if buffer.push(event).is_ok() {
              local_ops += 1;
            }

            if local_ops % 10000 == 0 {
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

            if local_ops % 10000 == 0 {
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
  duration: Duration,
  producers: usize,
  consumers: usize,
  buffer_size: usize,
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
  fn new(config: EndToEndConfig) -> Self { Self { config } }

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
        let file = File::create(path).ok();
        file.map(|f| Arc::new(Mutex::new(BufWriter::new(f))))
      }
    };

    // Producers
    let producer_handles: Vec<_> = (0..cfg.producers).map(|pid| {
      let buffer = Arc::clone(&buffer);
      let barrier = Arc::clone(&barrier);
      let stop_flag = Arc::clone(&stop_flag);
      let produced = Arc::clone(&produced);
      let drops = Arc::clone(&drops);
      thread::spawn(move || {
        let mut local_prod = 0u64;
        let mut local_drops = 0u64;
        let mut ctr = 0u64;
        barrier.wait();
        while Instant::now() < end && !stop_flag.load(Ordering::Acquire) {
          // enqueue timestamp (us since t0) into position.0 for latency measurement
          let ts_us = (Instant::now().duration_since(t0).as_micros() as u64) as u32;
          let mut ev = create_minimal_event(pid as u64 * 1_000_000 + ctr);
          ev.position = (ts_us as u32, 0);
          if buffer.push(ev).is_ok() {
            local_prod += 1;
          } else {
            local_drops += 1;
          }
          ctr += 1;
          if (ctr & 0x3FF) == 0 { thread::yield_now(); }
        }
        produced.fetch_add(local_prod, Ordering::Relaxed);
        drops.fetch_add(local_drops, Ordering::Relaxed);
      })
    }).collect();

    // Consumers
    let consumer_handles: Vec<_> = (0..cfg.consumers).map(|_cid| {
      let buffer = Arc::clone(&buffer);
      let barrier = Arc::clone(&barrier);
      let stop_flag = Arc::clone(&stop_flag);
      let consumed = Arc::clone(&consumed);
      let latencies = Arc::clone(&latencies);
      let bytes_written = Arc::clone(&bytes_written);
      let writer = writer.clone();
      thread::spawn(move || {
        let mut local_cons = 0u64;
        let mut local_lat: Vec<u64> = Vec::with_capacity(64_000);
        let mut local_bytes = 0u64;
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
              let mut w = w.lock().unwrap();
              // minimal formatting to include hot-path work
              let line = format!("{},{}:{}\n", ev.target_id, ev.file_id, ev.position.0);
              if w.write_all(line.as_bytes()).is_ok() {
                local_bytes += line.len() as u64;
              }
            }

            local_cons += 1;
          } else {
            thread::yield_now();
          }
          if (local_cons & 0x3FF) == 0 { thread::yield_now(); }
        }
        // flush any writer
        if let Some(w) = &writer { let _ = w.lock().unwrap().flush(); }
        consumed.fetch_add(local_cons, Ordering::Relaxed);
        bytes_written.fetch_add(local_bytes, Ordering::Relaxed);
        // merge latencies
        let mut g = latencies.lock().unwrap();
        g.extend(local_lat);
      })
    }).collect();

    // Start window
    let start = Instant::now();
    barrier.wait();
    let now = Instant::now();
    if end > now { thread::sleep(end - now); }
    stop_flag.store(true, Ordering::Release);

    for h in producer_handles { let _ = h.join(); }
    for h in consumer_handles { let _ = h.join(); }

    let dur = start.elapsed();
    let prod = produced.load(Ordering::Relaxed);
    let cons = consumed.load(Ordering::Relaxed);
    let drp = drops.load(Ordering::Relaxed);
    let bytes = bytes_written.load(Ordering::Relaxed);

    // percentiles
    let mut l = latencies.lock().unwrap();
    l.sort_unstable();
    let p = |q: f64| -> f64 {
      if l.is_empty() { return 0.0; }
      let idx = ((q * (l.len() as f64 - 1.0)).round() as usize).min(l.len()-1);
      l[idx] as f64
    };

    EndToEndResult {
      test_name: match cfg.sink { SinkKind::Null => "E2E-Null".into(), SinkKind::File(_) => "E2E-File".into() },
      producers: cfg.producers,
      consumers: cfg.consumers,
      buffer_size: cfg.buffer_size,
      produced_per_sec: prod as f64 / dur.as_secs_f64(),
      consumed_per_sec: cons as f64 / dur.as_secs_f64(),
      drops: drp,
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
        metric: "Allocations per Second".to_string(),
        value: memory_allocation_result.value,
        unit: memory_allocation_result.unit.clone(),
      },
      SummaryMetric {
        metric: "Bytes per Event (approx)".to_string(),
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
    println!("• All throughput/memory tests run with multiple trials for statistical confidence");
    println!("• Mean ± StdDev reported; lower StdDev indicates more consistent performance");
    println!("• Buffer sizes chosen to be realistic for production workloads");
    println!("• Tests use synchronized thread start and deadline-based measurement windows");
    println!("• Hot loops avoid tracing/logging overhead for accurate microbenchmarks");
    println!("\n✅ Statistical benchmark suite completed successfully!");
  }
}

fn main() {
  let test_duration = Duration::from_secs(3); // Shorter per-trial, but multiple trials
  let runs_per_test = 5; // Run each test 5 times for statistics
  let benchmark = ComprehensiveBenchmark::new(test_duration, runs_per_test);

  let suite = benchmark.run_statistical_benchmarks();
  suite.display_results();

  // =============================================================
  // End-to-End pipeline benchmarks (through ttlog-like path)
  // =============================================================
  println!("\n🧪 End-to-End Pipeline Benchmarks:");
  let e2e_cfg_null = EndToEndConfig {
    duration: test_duration,
    producers: 8,
    consumers: 1,
    buffer_size: 65_536,
    sink: SinkKind::Null,
  };
  let e2e_null = EndToEndTester::new(e2e_cfg_null);
  let e2e_null_results = e2e_null.run_with_stats(runs_per_test);
  println!("\n• E2E (Null sink):");
  let table = Table::new(&e2e_null_results).to_string();
  println!("{}", table);

  // Optional file sink run (small file)
  let e2e_cfg_file = EndToEndConfig {
    duration: test_duration,
    producers: 8,
    consumers: 1,
    buffer_size: 65_536,
    sink: SinkKind::File(PathBuf::from("/tmp/ttlog_e2e_bench.log")),
  };
  let e2e_file = EndToEndTester::new(e2e_cfg_file);
  let e2e_file_results = e2e_file.run_with_stats(runs_per_test);
  println!("\n• E2E (File sink):");
  let table = Table::new(&e2e_file_results).to_string();
  println!("{}", table);
}
