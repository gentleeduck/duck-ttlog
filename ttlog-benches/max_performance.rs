use std::borrow::Cow;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono;
use smallvec;
use tabled::{Table, Tabled};
use ttlog::{
  event::{Field, FieldValue, LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
  snapshot::SnapshotWriter,
  trace::Trace,
};

#[cfg(feature = "jemalloc")]
use jemalloc_ctl as jemctl;
#[cfg(feature = "sysinfo")]
use sysinfo::{System, SystemExt, RefreshKind, MemoryRefreshKind};

// ============================================================================
// Table Output Utilities
// ============================================================================

/// Represents a performance test result
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
  #[tabled(rename = "Additional Info")]
  additional_info: String,
}

/// Represents a summary metric
#[derive(Debug, Clone, Tabled)]
struct SummaryMetric {
  #[tabled(rename = "Metric")]
  metric: String,
  #[tabled(rename = "Value")]
  value: f64,
  #[tabled(rename = "Unit")]
  unit: String,
}

#[derive(Debug, Clone, Tabled)]
struct MemoryMatrixRow {
  #[tabled(rename = "Events")]
  events: usize,
  #[tabled(rename = "Fields/Event")]
  fields_per_event: usize,
  #[tabled(rename = "Msg Size (B)")]
  message_size_bytes: usize,
  #[tabled(rename = "Total Bytes (approx)")]
  total_bytes_approx: usize,
  #[tabled(rename = "Bytes/Event (approx)")]
  bytes_per_event_approx: usize,
  #[tabled(rename = "RSS Delta (MiB)")]
  rss_delta_mib: f64,
  #[tabled(rename = "Alloc Bytes (opt)")]
  alloc_bytes_opt: String,
}

/// Formats a table of test results using tabled
fn print_results_table(results: &[TestResult], title: &str) {
  println!("\n{}", "=".repeat(100));
  println!("{}", title);
  println!("{}", "=".repeat(100));

  if results.is_empty() {
    println!("No results to display");
    return;
  }

  let table = Table::new(results).to_string();
  println!("{}", table);
  println!("{}", "=".repeat(100));
}

/// Formats a summary table using tabled
fn print_summary_table(summary: &[SummaryMetric], title: &str) {
  println!("\n{}", "=".repeat(80));
  println!("{}", title);
  println!("{}", "=".repeat(80));

  if summary.is_empty() {
    println!("No summary to display");
    return;
  }

  let table = Table::new(summary).to_string();
  println!("{}", table);
  println!("{}", "=".repeat(80));
}

fn current_thread_id_u64() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}

// ============================================================================
// Maximum Performance Testing Components
// ============================================================================

/// Try to read current resident set size in bytes using sysinfo (if enabled)
fn read_rss_bytes() -> Option<u64> {
  #[cfg(feature = "sysinfo")]
  {
    let mut sys = System::new_with_specifics(RefreshKind::new().with_memory(MemoryRefreshKind::everything()));
    sys.refresh_memory();
    return Some(sys.process(sysinfo::get_current_pid().ok()?)?.memory() * 1024);
  }
  None
}

/// Try to read jemalloc allocated bytes (if enabled)
fn read_jemalloc_allocated_bytes() -> Option<u64> {
  #[cfg(feature = "jemalloc")]
  {
    let epoch = jemctl::epoch::mib().ok()?;
    let allocated = jemctl::stats::allocated::mib().ok()?;
    let _ = epoch.advance();
    return allocated.read().ok();
  }
  None
}

/// Tests maximum throughput capabilities
struct ThroughputTester {
  test_duration: Duration,
  event_count: Arc<AtomicU64>,
}

impl ThroughputTester {
  fn new(test_duration: Duration) -> Self {
    Self {
      test_duration,
      event_count: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Test maximum events per second
  fn max_events_per_second(&self, buffer_size: usize) -> TestResult {
    let start = Instant::now();
    let _trace_system = Trace::init(buffer_size, buffer_size / 10);

    let event_count = Arc::clone(&self.event_count);
    let test_duration = self.test_duration;

    // Create multiple high-frequency logging threads
    let handles: Vec<_> = (0..16)
      .map(|thread_id| {
        let event_count = Arc::clone(&event_count);
        thread::spawn(move || {
          let mut local_count = 0;
          let thread_start = Instant::now();

          while thread_start.elapsed() < test_duration {
            tracing::info!(
                thread_id = thread_id,
                event_id = local_count,
                timestamp = %chrono::Utc::now(),
                "High frequency event"
            );
            local_count += 1;
            event_count.fetch_add(1, Ordering::Relaxed);
          }

          local_count
        })
      })
      .collect();

    // Wait for completion
    for handle in handles {
      handle.join().unwrap();
    }

    let total_duration = start.elapsed();
    let total_events = self.event_count.load(Ordering::Relaxed);
    let events_per_second = total_events as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "High Frequency Events".to_string(),
      metric: "Events per Second".to_string(),
      value: events_per_second,
      unit: "events/sec".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: format!(
        "Total Events: {}, Threads: 16, Buffer Size: {}",
        total_events, buffer_size
      ),
    }
  }

  /// Test maximum buffer operations per second
  fn max_buffer_operations(&self, buffer_size: usize) -> TestResult {
    let start = Instant::now();
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size));

    let operation_count = Arc::new(AtomicU64::new(0));
    let test_duration = self.test_duration;

    // Create producer and consumer threads
    let producer_handles: Vec<_> = (0..8)
      .map(|producer_id| {
        let buffer = Arc::clone(&buffer);
        let operation_count = Arc::clone(&operation_count);
        thread::spawn(move || {
          let mut local_ops = 0;
          let thread_start = Instant::now();

          while thread_start.elapsed() < test_duration {
            let event = create_performance_event(producer_id, local_ops);
            if buffer.push(event).is_ok() {
              local_ops += 1;
              operation_count.fetch_add(1, Ordering::Relaxed);
            }
          }

          local_ops
        })
      })
      .collect();

    let consumer_handles: Vec<_> = (0..4)
      .map(|_consumer_id| {
        let buffer = Arc::clone(&buffer);
        let operation_count = Arc::clone(&operation_count);
        thread::spawn(move || {
          let mut local_ops = 0;
          let thread_start = Instant::now();

          while thread_start.elapsed() < test_duration {
            if let Some(_) = buffer.pop() {
              local_ops += 1;
              operation_count.fetch_add(1, Ordering::Relaxed);
            }
          }

          local_ops
        })
      })
      .collect();

    // Wait for completion
    for handle in producer_handles {
      handle.join().unwrap();
    }
    for handle in consumer_handles {
      handle.join().unwrap();
    }

    let total_duration = start.elapsed();
    let total_operations = operation_count.load(Ordering::Relaxed);
    let ops_per_second = total_operations as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "Buffer Operations".to_string(),
      metric: "Operations per Second".to_string(),
      value: ops_per_second,
      unit: "ops/sec".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: format!(
        "Total Operations: {}, Producers: 8, Consumers: 4, Buffer Size: {}",
        total_operations, buffer_size
      ),
    }
  }
}

/// Tests maximum concurrency capabilities
struct ConcurrencyTester;

impl ConcurrencyTester {
  fn new(_test_duration: Duration) -> Self {
    Self
  }

  /// Test maximum concurrent threads
  fn max_concurrent_threads(&self, max_threads: usize) -> TestResult {
    let start = Instant::now();
    let mut successful_threads = 0;
    let mut test_results = Vec::new();

    for thread_count in [1, 2, 4, 8, 16, 32, 64, 128, 256, 512, 1024].iter() {
      if *thread_count > max_threads {
        break;
      }

      let test_start = Instant::now();
      let mut handles = Vec::new();

      // Create test threads
      for i in 0..*thread_count {
        let handle = thread::spawn(move || {
          let mut local_count = 0;
          for j in 0..1000 {
            // Simulate some work
            let _ = (i * j).wrapping_mul(7).rotate_left(3);
            local_count += 1;
          }
          local_count
        });
        handles.push(handle);
      }

      // Wait for completion
      let mut all_successful = true;
      for handle in handles {
        if handle.join().is_err() {
          all_successful = false;
          break;
        }
      }

      let thread_duration = test_start.elapsed();

      if all_successful && thread_duration < Duration::from_secs(30) {
        successful_threads = *thread_count;
        test_results.push(format!(
          "✅ {} threads: {:?}",
          thread_count, thread_duration
        ));
      } else {
        test_results.push(format!(
          "❌ {} threads: {:?}",
          thread_count, thread_duration
        ));
        break;
      }
    }

    let total_duration = start.elapsed();

    TestResult {
      test_name: "Concurrent Threads".to_string(),
      metric: "Maximum Threads".to_string(),
      value: successful_threads as f64,
      unit: "threads".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: test_results.join(", "),
    }
  }

  /// Test maximum concurrent buffers
  fn max_concurrent_buffers(&self, max_buffers: usize) -> TestResult {
    let start = Instant::now();
    let mut successful_buffers = 0;
    let mut test_results = Vec::new();

    for buffer_count in [1, 10, 100, 1000, 10000, 100000].iter() {
      if *buffer_count > max_buffers {
        break;
      }

      let test_start = Instant::now();
      let mut buffers = Vec::new();

      // Create test buffers
      for _i in 0..*buffer_count {
        let buffer = LockFreeRingBuffer::<i32>::new(1000);
        buffers.push(buffer);
      }

      // Test operations on all buffers
      let mut all_successful = true;
      for (i, buffer) in buffers.iter().enumerate() {
        for j in 0..100 {
          if buffer.push((i * 100 + j) as i32).is_err() {
            all_successful = false;
            break;
          }
        }
        if !all_successful {
          break;
        }
      }

      let buffer_duration = test_start.elapsed();

      if all_successful && buffer_duration < Duration::from_secs(30) {
        successful_buffers = *buffer_count;
        test_results.push(format!(
          "✅ {} buffers: {:?}",
          buffer_count, buffer_duration
        ));
      } else {
        test_results.push(format!(
          "❌ {} buffers: {:?}",
          buffer_count, buffer_duration
        ));
        break;
      }
    }

    let total_duration = start.elapsed();

    TestResult {
      test_name: "Concurrent Buffers".to_string(),
      metric: "Maximum Buffers".to_string(),
      value: successful_buffers as f64,
      unit: "buffers".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: test_results.join(", "),
    }
  }
}

/// Tests maximum memory efficiency
struct MemoryEfficiencyTester;

impl MemoryEfficiencyTester {
  fn new(_test_duration: Duration) -> Self {
    Self
  }

  /// More accurate estimation: include alignment padding, string capacities, and fields
  fn estimate_event_bytes(event: &LogEvent) -> usize {
    // Base struct size
    let base = std::mem::size_of::<LogEvent>();
    // Target/message approximations (use len as lower bound)
    let target = event.target.len();
    let message = event.message.len();
    // Fields: size of Field plus enum payload; approximate with key length and match arms
    let mut fields_bytes = 0usize;
    for f in &event.fields {
      fields_bytes += std::mem::size_of::<Field>();
      fields_bytes += f.key.len();
      // Payload approx
      fields_bytes += match &f.value {
        FieldValue::Str(s) => s.len(),
        FieldValue::String(s) => s.len(),
        FieldValue::Debug(s) | FieldValue::Display(s) => s.len(),
        _ => std::mem::size_of_val(&f.value),
      };
    }
    // Rough alignment padding to 8 bytes
    let total = base + target + message + fields_bytes;
    (total + 7) & !7
  }

  /// Test memory matrix across (events x fields x message size)
  fn memory_matrix(&self) -> Vec<MemoryMatrixRow> {
    let mut rows = Vec::new();

    let event_counts = [1_000usize, 10_000, 100_000];
    let fields_options = [0usize, 3, 8];
    let message_sizes = [32usize, 128, 512];

    // Baseline RSS / jemalloc allocated
    let rss_before = read_rss_bytes();
    let alloc_before = read_jemalloc_allocated_bytes();

    for &count in &event_counts {
      for &fields_per_event in &fields_options {
        for &msg_size in &message_sizes {
          // Build synthetic events
          let mut events = Vec::with_capacity(count);
          let mut approx_total = 0usize;
          for i in 0..count {
            let mut fields = smallvec::smallvec![];
            for j in 0..fields_per_event {
              fields.push(Field {
                key: Cow::Owned(format!("k{}", j)),
                value: match j % 6 {
                  0 => FieldValue::U64(i as u64),
                  1 => FieldValue::I64((i as i64) * 7),
                  2 => FieldValue::F64((i as f64) * 3.14),
                  3 => FieldValue::Bool(i % 2 == 0),
                  4 => FieldValue::Str(Cow::Owned("x".repeat(8))),
                  _ => FieldValue::Debug(format!("dbg-{}-{}", i, j)),
                },
              });
            }

            let event = LogEvent {
              timestamp_nanos: i as u64,
              level: LogLevel::Info,
              target: Cow::Borrowed("matrix"),
              message: Cow::Owned("m".repeat(msg_size)),
              fields,
              thread_id: 0,
              file: None,
              line: None,
            };
            approx_total += Self::estimate_event_bytes(&event);
            events.push(event);
          }

          // Touch the data to prevent optimizations
          let checksum: u64 = events.iter().map(|e| e.message.len() as u64).sum();

          // Measure deltas
          let rss_after = read_rss_bytes();
          let alloc_after = read_jemalloc_allocated_bytes();
          let rss_delta_mib = match (rss_before, rss_after) {
            (Some(b), Some(a)) if a > b => (a - b) as f64 / (1024.0 * 1024.0),
            _ => 0.0,
          };
          let alloc_delta = match (alloc_before, alloc_after) {
            (Some(b), Some(a)) if a > b => format!("{}", a - b),
            _ => "-".to_string(),
          };

          rows.push(MemoryMatrixRow {
            events: count,
            fields_per_event,
            message_size_bytes: msg_size,
            total_bytes_approx: approx_total,
            bytes_per_event_approx: approx_total / count.max(1),
            rss_delta_mib,
            alloc_bytes_opt: alloc_delta,
          });

          // Free
          drop(events);
        }
      }
    }

    rows
  }

  /// Test maximum memory efficiency
  fn max_memory_efficiency(&self) -> TestResult {
    let start = Instant::now();

    // Build a small controlled set to estimate bytes per event
    let mut total_memory = 0usize;
    let mut total_events = 0usize;
    let mut test_details = Vec::new();

    for event_size in [100, 1_000, 10_000].iter() {
      let test_start = Instant::now();
      let mut events: Vec<LogEvent> = (0..*event_size)
        .map(|i| create_large_event(i as u64))
        .collect();

      // Estimate memory (struct + strings + fields, with alignment)
      let mut memory_usage = 0usize;
      for event in &events {
        memory_usage += Self::estimate_event_bytes(event);
      }
      total_memory += memory_usage;
      total_events += *event_size;

      let test_duration = test_start.elapsed();
      test_details.push(format!(
        "{} events: {} approx bytes in {:?}",
        event_size, memory_usage, test_duration
      ));

      // Ensure events are used
      events.retain(|e| e.message.len() > 0);
    }

    let total_duration = start.elapsed();
    let memory_per_event = if total_events > 0 {
      total_memory as f64 / total_events as f64
    } else {
      0.0
    };

    TestResult {
      test_name: "Memory Efficiency".to_string(),
      metric: "Memory per Event (approx)".to_string(),
      value: memory_per_event,
      unit: "bytes/event".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: test_details.join(", "),
    }
  }

  /// Test maximum snapshot performance
  fn max_snapshot_performance(&self) -> TestResult {
    let start = Instant::now();

    let mut total_snapshots = 0;
    let mut total_events = 0;
    let mut test_details = Vec::new();

    for event_count in [1_000, 10_000, 100_000].iter() {
      let test_start = Instant::now();

      // Create buffer with events
      let mut buffer = LockFreeRingBuffer::<LogEvent>::new(*event_count);
      for i in 0..*event_count {
        let event = create_performance_event(0, i as u64);
        buffer.push(event).unwrap();
      }

      // Create snapshot
      let writer = SnapshotWriter::new("max_performance_test");
      if let Some(_) = writer.create_snapshot(&mut buffer, "performance_test") {
        total_snapshots += 1;
        total_events += *event_count;
      }

      let test_duration = test_start.elapsed();
      test_details.push(format!("{} events: {:?}", event_count, test_duration));
    }

    let total_duration = start.elapsed();
    let events_per_second = total_events as f64 / total_duration.as_secs_f64();

    TestResult {
      test_name: "Snapshot Performance".to_string(),
      metric: "Events per Second".to_string(),
      value: events_per_second,
      unit: "events/sec".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: format!(
        "Total Snapshots: {}, Total Events: {}, {}",
        total_snapshots,
        total_events,
        test_details.join(", ")
      ),
    }
  }
}

// ============================================================================
// Main Performance Testing Functions
// ============================================================================

fn run_memory_efficiency_tests() {
  println!("🚀 Starting Maximum Memory Efficiency Tests...");
  println!("==============================================");

  let tester = MemoryEfficiencyTester::new(Duration::from_secs(30));
  let mut results = Vec::new();

  // Test 1: Maximum memory efficiency (approximate per event)
  println!("\n📊 Test 1: Memory Efficiency (Approx)");
  let memory_efficiency_result = tester.max_memory_efficiency();
  results.push(memory_efficiency_result.clone());

  // Test 2: Memory Matrix (events x fields x message size)
  println!("\n📊 Test 2: Memory Matrix (Events × Fields × Message Size)");
  let matrix = tester.memory_matrix();
  let table = Table::new(&matrix).to_string();
  println!("{}", table);

  // Display results in table format
  print_results_table(&results, "🚀 MEMORY EFFICIENCY TEST RESULTS");

  // Create summary
  let summary = vec![SummaryMetric {
    metric: "Approx Bytes per Event".to_string(),
    value: results
      .iter()
      .find(|r| r.test_name == "Memory Efficiency")
      .map(|r| r.value)
      .unwrap_or(0.0),
    unit: "bytes/event".to_string(),
  }];
  print_summary_table(&summary, "📊 MEMORY EFFICIENCY SUMMARY");

  println!("\n🎉 Memory efficiency tests completed!");
}

fn run_throughput_tests() {
  println!("🚀 Starting Maximum Throughput Tests...");
  println!("=======================================");

  let tester = ThroughputTester::new(Duration::from_secs(10));
  let mut results = Vec::new();

  // Test 1: Maximum events per second
  println!("\n📊 Test 1: Maximum Events Per Second");
  let max_events_result = tester.max_events_per_second(1_000_000);
  results.push(max_events_result.clone());

  // Test 2: Maximum buffer operations
  println!("\n📊 Test 2: Maximum Buffer Operations");
  let max_ops_result = tester.max_buffer_operations(1_000_000);
  results.push(max_ops_result.clone());

  // Display results in table format
  print_results_table(&results, "🚀 THROUGHPUT TEST RESULTS");

  // Create summary
  let summary = vec![
    SummaryMetric {
      metric: "Maximum Events per Second".to_string(),
      value: results[0].value,
      unit: results[0].unit.clone(),
    },
    SummaryMetric {
      metric: "Maximum Buffer Operations per Second".to_string(),
      value: results[1].value,
      unit: results[1].unit.clone(),
    },
  ];
  print_summary_table(&summary, "📊 THROUGHPUT SUMMARY");

  println!("\n🎉 Throughput tests completed!");
}

fn run_concurrency_tests() {
  println!("🚀 Starting Maximum Concurrency Tests...");
  println!("========================================");

  let tester = ConcurrencyTester::new(Duration::from_secs(60));
  let mut results = Vec::new();

  // Test 1: Maximum concurrent threads
  println!("\n📊 Test 1: Maximum Concurrent Threads");
  let max_threads_result = tester.max_concurrent_threads(1024);
  results.push(max_threads_result.clone());

  // Test 2: Maximum concurrent buffers
  println!("\n📊 Test 2: Maximum Concurrent Buffers");
  let max_buffers_result = tester.max_concurrent_buffers(100_000);
  results.push(max_buffers_result.clone());

  // Display results in table format
  print_results_table(&results, "🚀 CONCURRENCY TEST RESULTS");

  // Create summary
  let summary = vec![
    SummaryMetric {
      metric: "Maximum Concurrent Threads".to_string(),
      value: results[0].value,
      unit: results[0].unit.clone(),
    },
    SummaryMetric {
      metric: "Maximum Concurrent Buffers".to_string(),
      value: results[1].value,
      unit: results[1].unit.clone(),
    },
  ];
  print_summary_table(&summary, "📊 CONCURRENCY SUMMARY");

  println!("\n🎉 Concurrency tests completed!");
}

fn run_comprehensive_performance_tests() {
  println!("🚀 Starting Comprehensive Maximum Performance Tests...");
  println!("=====================================================");

  let start = Instant::now();
  let mut all_results = Vec::new();
  let mut all_summaries = Vec::new();

  // Run throughput tests and collect results
  println!("\n📊 Running Throughput Tests...");
  let tester_throughput = ThroughputTester::new(Duration::from_secs(10));
  let max_events_result = tester_throughput.max_events_per_second(1_000_000);
  let max_ops_result = tester_throughput.max_buffer_operations(1_000_000);
  all_results.extend_from_slice(&[max_events_result.clone(), max_ops_result.clone()]);
  all_summaries.extend_from_slice(&[
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

  // Run concurrency tests and collect results
  println!("\n📊 Running Concurrency Tests...");
  let tester_concurrency = ConcurrencyTester::new(Duration::from_secs(60));
  let max_threads_result = tester_concurrency.max_concurrent_threads(1024);
  let max_buffers_result = tester_concurrency.max_concurrent_buffers(100_000);
  all_results.extend_from_slice(&[max_threads_result.clone(), max_buffers_result.clone()]);
  all_summaries.extend_from_slice(&[
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

  // Run memory efficiency tests and collect results
  println!("\n📊 Running Memory Efficiency Tests...");
  let tester_memory = MemoryEfficiencyTester::new(Duration::from_secs(30));
  let memory_efficiency_result = tester_memory.max_memory_efficiency();
  all_results.push(memory_efficiency_result.clone());
  all_summaries.push(SummaryMetric {
    metric: "Approx Bytes per Event".to_string(),
    value: memory_efficiency_result.value,
    unit: memory_efficiency_result.unit.clone(),
  });

  let total_duration = start.elapsed();

  // Display comprehensive results table
  print_results_table(&all_results, "🚀 COMPREHENSIVE PERFORMANCE TEST RESULTS");

  // Display grand summary
  print_summary_table(&all_summaries, "📊 GRAND PERFORMANCE SUMMARY");

  // Display final statistics
  println!("\n{}", "=".repeat(80));
  println!("🎉 COMPREHENSIVE PERFORMANCE TEST COMPLETED!");
  println!("{}", "=".repeat(80));
  println!("🚀 Total Test Duration: {:?}", total_duration);
  println!("🚀 Total Tests Run: {}", all_results.len());
  println!("🚀 TTLog has been tested at its absolute performance limits!");
  println!("{}", "=".repeat(80));
}

// ============================================================================
// Utility Functions
// ============================================================================

fn create_performance_event(thread_id: u32, event_id: u64) -> LogEvent {
  LogEvent {
    timestamp_nanos: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    level: LogLevel::Info,
    target: Cow::Borrowed("max_performance"),
    message: Cow::Owned(format!(
      "Performance event {} from thread {}",
      event_id, thread_id
    )),
    fields: smallvec::smallvec![
      Field {
        key: "thread_id".into(),
        value: FieldValue::U64(thread_id as u64),
      },
      Field {
        key: "event_id".into(),
        value: FieldValue::U64(event_id),
      },
      Field {
        key: "timestamp".into(),
        value: FieldValue::U64(chrono::Utc::now().timestamp_millis() as u64),
      },
      Field {
        key: "performance_level".into(),
        value: FieldValue::Str("maximum".into()),
      },
    ],
    thread_id: current_thread_id_u64(),
    file: Some("max_performance.rs".into()),
    line: Some(42),
  }
}

fn create_large_event(event_id: u64) -> LogEvent {
  LogEvent {
    timestamp_nanos: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    level: LogLevel::Info,
    target: Cow::Borrowed("large_event"),
    message: Cow::Owned(format!("Large event {} with extensive fields", event_id)),
    fields: smallvec::smallvec![
      Field {
        key: "event_id".into(),
        value: FieldValue::U64(event_id),
      },
      Field {
        key: "large_string_1".into(),
        value: FieldValue::Str(
          "This is a very long string field that takes up significant memory space".into()
        ),
      },
      Field {
        key: "large_string_2".into(),
        value: FieldValue::Str(
          "Another very long string field to increase memory usage for testing purposes".into()
        ),
      },
      Field {
        key: "large_string_3".into(),
        value: FieldValue::Str(
          "Yet another long string field to maximize memory consumption during testing".into()
        ),
      },
      Field {
        key: "numeric_field_1".into(),
        value: FieldValue::I64(event_id as i64 * 1000),
      },
      Field {
        key: "numeric_field_2".into(),
        value: FieldValue::U64(event_id * 2000),
      },
      Field {
        key: "numeric_field_3".into(),
        value: FieldValue::F64(event_id as f64 * 3.14159),
      },
      Field {
        key: "boolean_field_1".into(),
        value: FieldValue::Bool(event_id % 2 == 0),
      },
      Field {
        key: "boolean_field_2".into(),
        value: FieldValue::Bool(event_id % 3 == 0),
      },
      Field {
        key: "debug_field".into(),
        value: FieldValue::Debug(format!(
          "Complex debug information for event {} with additional context",
          event_id
        )),
      },
    ],
    thread_id: current_thread_id_u64(),
    file: Some("max_performance.rs".into()),
    line: Some(42),
  }
}

// ============================================================================
// Main Function
// ============================================================================

fn main() {
  println!("🚀 TTLog Maximum Performance Testing Suite");
  println!("=========================================");
  println!("This suite tests TTLog at its absolute performance limits!");
  println!();

  // Parse command line arguments for test selection
  let args: Vec<String> = std::env::args().collect();

  if args.len() > 1 {
    match args[1].as_str() {
      "throughput" => {
        println!("🎯 Running Throughput Tests Only");
        run_throughput_tests();
      },
      "concurrency" => {
        println!("🎯 Running Concurrency Tests Only");
        run_concurrency_tests();
      },
      "memory" => {
        println!("🎯 Running Memory Efficiency Tests Only");
        run_memory_efficiency_tests();
      },
      "all" | _ => {
        println!("🎯 Running All Performance Tests");
        run_comprehensive_performance_tests();
      },
    }
  } else {
    println!("🎯 Running All Performance Tests (default)");
    run_comprehensive_performance_tests();
  }

  println!("\n🚀 Maximum performance testing completed!");
  println!("🚀 TTLog has proven its performance capabilities at the limit!");
}
