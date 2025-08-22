use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use tabled::{Table, Tabled};
use ttlog::{
  event::{LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
  string_interner::StringInterner,
  trace::Trace,
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

  /// Test maximum events per second
  fn max_events_per_second(&self, buffer_size: usize) -> TestResult {
    let start = Instant::now();
    let _trace_system = Trace::init(buffer_size, buffer_size / 10, "test", Some("/tmp/"));
    let event_count = Arc::new(AtomicU64::new(0));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let thread_count = 16; // Optimal thread count for maximum throughput

    let handles: Vec<_> = (0..thread_count)
      .map(|thread_id| {
        let event_count = Arc::clone(&event_count);
        let stop_flag = Arc::clone(&stop_flag);

        thread::spawn(move || {
          let mut local_count = 0u64;

          while !stop_flag.load(Ordering::Relaxed) {
            tracing::info!(
              thread_id = thread_id,
              event_id = local_count,
              "High frequency event"
            );
            local_count += 1;
            event_count.fetch_add(1, Ordering::Relaxed);

            if local_count % 10000 == 0 {
              thread::yield_now();
            }
          }

          local_count
        })
      })
      .collect();

    thread::sleep(self.test_duration);
    stop_flag.store(true, Ordering::Relaxed);

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
    let start = Instant::now();
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let total_ops = Arc::new(AtomicU64::new(0));
    let thread_count = 8; // Balanced for buffer operations

    let handles: Vec<_> = (0..thread_count)
      .map(|thread_id| {
        let buffer = Arc::clone(&buffer);
        let stop_flag = Arc::clone(&stop_flag);
        let total_ops = Arc::clone(&total_ops);

        thread::spawn(move || {
          let mut local_ops = 0u64;
          let mut counter = 0u64;

          while !stop_flag.load(Ordering::Relaxed) {
            let event = create_minimal_event(thread_id as u64 * 1000000 + counter);
            counter += 1;

            // Push operation
            if buffer.push(event).is_ok() {
              local_ops += 1;
            }

            // Pop operation (every 10th iteration to maintain balance)
            if counter % 10 == 0 {
              if buffer.pop().is_some() {
                local_ops += 1;
              }
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

    thread::sleep(self.test_duration);
    stop_flag.store(true, Ordering::Relaxed);

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
      let stop_flag = Arc::new(AtomicBool::new(false));

      let handles: Vec<_> = (0..*thread_count)
        .map(|thread_id| {
          let total_ops = Arc::clone(&total_ops);
          let stop_flag = Arc::clone(&stop_flag);

          thread::spawn(move || {
            let mut local_ops = 0u64;
            let mut counter = 0u64;

            while !stop_flag.load(Ordering::Relaxed) {
              // Simulate work
              let hash = thread_id
                .wrapping_mul(counter as usize)
                .wrapping_add(0xdeadbeef);
              let _result = hash.rotate_left(7).wrapping_mul(0x9e3779b9);

              counter += 1;
              local_ops += 1;

              if local_ops % 1000 == 0 {
                thread::yield_now();
              }
            }

            total_ops.fetch_add(local_ops, Ordering::Relaxed);
            local_ops
          })
        })
        .collect();

      // Run for 100ms
      thread::sleep(Duration::from_millis(100));
      stop_flag.store(true, Ordering::Relaxed);

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

    for buffer_count in [1, 10, 100, 1000, 10000, 100000].iter() {
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
    let start = Instant::now();
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(100000));
    let stop_flag = Arc::new(AtomicBool::new(false));
    let total_bytes = Arc::new(AtomicU64::new(0));
    let thread_count = 8;

    let handles: Vec<_> = (0..thread_count)
      .map(|thread_id| {
        let buffer = Arc::clone(&buffer);
        let stop_flag = Arc::clone(&stop_flag);
        let total_bytes = Arc::clone(&total_bytes);

        thread::spawn(move || {
          let mut local_bytes = 0u64;
          let mut counter = 0u64;

          while !stop_flag.load(Ordering::Relaxed) {
            let event = create_minimal_event(thread_id as u64 * 1000000 + counter);
            let event_size = Self::estimate_event_size() as u64;

            if buffer.push(event).is_ok() {
              local_bytes += event_size;
              counter += 1;
            }

            // Consume occasionally
            if counter % 100 == 0 {
              if buffer.pop().is_some() {
                // Event consumed
              }
            }

            if counter % 1000 == 0 {
              thread::yield_now();
            }
          }

          total_bytes.fetch_add(local_bytes, Ordering::Relaxed);
          local_bytes
        })
      })
      .collect();

    thread::sleep(self.test_duration);
    stop_flag.store(true, Ordering::Relaxed);

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
    let start = Instant::now();
    let stop_flag = Arc::new(AtomicBool::new(false));
    let total_ops = Arc::new(AtomicU64::new(0));

    // Producer threads
    let producer_handles: Vec<_> = (0..producers)
      .map(|producer_id| {
        let buffer = Arc::clone(&buffer);
        let stop_flag = Arc::clone(&stop_flag);
        let total_ops = Arc::clone(&total_ops);

        thread::spawn(move || {
          let mut local_ops = 0u64;
          let mut event_counter = 0u64;

          while !stop_flag.load(Ordering::Relaxed) {
            let event = create_minimal_event(producer_id as u64 * 1000000 + event_counter);
            event_counter += 1;

            buffer.push_overwrite(event);
            local_ops += 1;

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

        thread::spawn(move || {
          let mut local_ops = 0u64;

          while !stop_flag.load(Ordering::Relaxed) {
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

    thread::sleep(self.test_duration);
    stop_flag.store(true, Ordering::Relaxed);

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
// Utility Functions
// ============================================================================

fn create_minimal_event(counter: u64) -> LogEvent {
  static INTERNER: std::sync::OnceLock<Arc<StringInterner>> = std::sync::OnceLock::new();
  let interner = INTERNER.get_or_init(|| Arc::new(StringInterner::new()));

  // Intern common strings
  let target_id = interner.intern_target("bench");
  let message_id = Some(interner.intern_message("test message"));
  let kv_id = Some(interner.intern_kv("test kv"));
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
    _padding: [0; 9],
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
  let message_id = Some(interner.intern_message(message));
  let file_id = interner.intern_file(file!());
  let kv_id = Some(interner.intern_kv("key"));

  let ts_ms = std::time::SystemTime::now()
    .duration_since(std::time::UNIX_EPOCH)
    .unwrap_or_else(|_| std::time::Duration::from_millis(0))
    .as_millis() as u64;
  let packed = LogEvent::pack_meta(ts_ms, LogLevel::INFO, 0);

  let mut ev = LogEvent {
    kv_id,
    packed_meta: packed,
    target_id,
    message_id,
    file_id,
    position: (0, 0),
    _padding: [0; 9],
  };

  let line = (counter & 0xFFFF) as u32;
  ev.position = (line, 0);
  ev
}

// ============================================================================
// Comprehensive Benchmark Runner
// ============================================================================

struct ComprehensiveBenchmark {
  test_duration: Duration,
}

impl ComprehensiveBenchmark {
  fn new(test_duration: Duration) -> Self {
    Self { test_duration }
  }

  /// Run all benchmarks and collect results
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

fn main() {
  println!("🔬 TTLog Maximum Performance Benchmark (Unified Output)");
  println!("==============================================");

  // Optional: allow duration override via env or args in the future; default to 5s
  let duration = Duration::from_secs(5);
  let runner = ComprehensiveBenchmark::new(duration);
  let (mut test_results, buffer_results, _summary) = runner.run_all_benchmarks();

  // Merge buffer tests into unified TestResult list
  let mut buffer_as_results = buffer_tests_to_results(&buffer_results);
  test_results.append(&mut buffer_as_results);

  // Print a single consolidated table of all results
  print_unified_results_table(&test_results);

  println!("\n✅ Benchmark completed. Results above.");
}
