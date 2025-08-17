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

  /// Test maximum memory efficiency
  fn max_memory_efficiency(&self) -> TestResult {
    let start = Instant::now();

    // Test with different event sizes
    let mut total_memory = 0;
    let mut total_events = 0;
    let mut test_details = Vec::new();

    for event_size in [100, 1000, 10000, 100000].iter() {
      let test_start = Instant::now();
      let mut events = Vec::new();

      // Create events of specified size
      for i in 0..*event_size {
        let event = create_large_event(i);
        events.push(event);
      }

      // Calculate actual memory usage by summing individual event sizes
      let mut memory_usage = 0;
      for event in &events {
        // Calculate size of the event structure
        let event_size_bytes = std::mem::size_of::<LogEvent>();
        // Add size of the message string
        let message_size = event.message.len();
        // Add size of fields (approximate)
        let fields_size = event.fields.len() * std::mem::size_of::<Field>();
        // Add size of target string
        let target_size = event.target.len();

        memory_usage += event_size_bytes + message_size + fields_size + target_size;
      }

      total_memory += memory_usage;
      total_events += *event_size;

      let test_duration = test_start.elapsed();
      test_details.push(format!(
        "{} events: {} bytes in {:?}",
        event_size, memory_usage, test_duration
      ));

      // Clean up
      events.clear();
    }

    let total_duration = start.elapsed();
    let memory_per_event = if total_events > 0 {
      total_memory as f64 / total_events as f64
    } else {
      0.0
    };

    let additional_info: Vec<(String, String)> = vec![
      (
        "Total Memory".to_string(),
        format!("{} bytes", total_memory),
      ),
      ("Total Events".to_string(), total_events.to_string()),
    ]
    .into_iter()
    .chain(test_details.into_iter().map(|d| ("Test".to_string(), d)))
    .collect();

    TestResult {
      test_name: "Memory Efficiency".to_string(),
      metric: "Memory per Event".to_string(),
      value: memory_per_event,
      unit: "bytes/event".to_string(),
      duration: format!("{:?}", total_duration),
      additional_info: additional_info
        .into_iter()
        .map(|(k, v)| format!("{}: {}", k, v))
        .collect::<Vec<_>>()
        .join(", "),
    }
  }

  /// Test maximum snapshot performance
  fn max_snapshot_performance(&self) -> TestResult {
    let start = Instant::now();

    let mut total_snapshots = 0;
    let mut total_events = 0;
    let mut test_details = Vec::new();

    for event_count in [1000, 10000, 100000].iter() {
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

fn run_throughput_tests() {
  println!("🚀 Starting Maximum Throughput Tests...");
  println!("=======================================");

  let tester = ThroughputTester::new(Duration::from_secs(10));
  let mut results = Vec::new();

  // Test 1: Maximum events per second
  println!("\n📊 Test 1: Maximum Events Per Second");
  let max_events_result = tester.max_events_per_second(1000000);
  results.push(max_events_result.clone());

  // Test 2: Maximum buffer operations
  println!("\n📊 Test 2: Maximum Buffer Operations");
  let max_ops_result = tester.max_buffer_operations(1000000);
  results.push(max_ops_result.clone());

  // Display results in table format
  print_results_table(&results, "🚀 THROUGHPUT TEST RESULTS");

  // Create summary
  let summary = vec![
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
  let max_buffers_result = tester.max_concurrent_buffers(100000);
  results.push(max_buffers_result.clone());

  // Display results in table format
  print_results_table(&results, "🚀 CONCURRENCY TEST RESULTS");

  // Create summary
  let summary = vec![
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
  ];
  print_summary_table(&summary, "📊 CONCURRENCY SUMMARY");

  println!("\n🎉 Concurrency tests completed!");
}

fn run_memory_efficiency_tests() {
  println!("🚀 Starting Maximum Memory Efficiency Tests...");
  println!("==============================================");

  let tester = MemoryEfficiencyTester::new(Duration::from_secs(30));
  let mut results = Vec::new();

  // Test 1: Maximum memory efficiency
  println!("\n📊 Test 1: Maximum Memory Efficiency");
  let memory_efficiency_result = tester.max_memory_efficiency();
  results.push(memory_efficiency_result.clone());

  // Test 2: Maximum snapshot performance
  println!("\n📊 Test 2: Maximum Snapshot Performance");
  let snapshot_performance_result = tester.max_snapshot_performance();
  results.push(snapshot_performance_result.clone());

  // Display results in table format
  print_results_table(&results, "🚀 MEMORY EFFICIENCY TEST RESULTS");

  // Create summary
  let summary = vec![
    SummaryMetric {
      metric: "Memory Efficiency".to_string(),
      value: memory_efficiency_result.value,
      unit: memory_efficiency_result.unit.clone(),
    },
    SummaryMetric {
      metric: "Snapshot Performance".to_string(),
      value: snapshot_performance_result.value,
      unit: snapshot_performance_result.unit.clone(),
    },
  ];
  print_summary_table(&summary, "📊 MEMORY EFFICIENCY SUMMARY");

  println!("\n🎉 Memory efficiency tests completed!");
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
  let max_events_result = tester_throughput.max_events_per_second(1000000);
  let max_ops_result = tester_throughput.max_buffer_operations(1000000);
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
  let max_buffers_result = tester_concurrency.max_concurrent_buffers(100000);
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
  let snapshot_performance_result = tester_memory.max_snapshot_performance();
  all_results.extend_from_slice(&[
    memory_efficiency_result.clone(),
    snapshot_performance_result.clone(),
  ]);
  all_summaries.extend_from_slice(&[
    SummaryMetric {
      metric: "Memory Efficiency".to_string(),
      value: memory_efficiency_result.value,
      unit: memory_efficiency_result.unit.clone(),
    },
    SummaryMetric {
      metric: "Snapshot Performance".to_string(),
      value: snapshot_performance_result.value,
      unit: snapshot_performance_result.unit.clone(),
    },
  ]);

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
    message: format!("Performance event {} from thread {}", event_id, thread_id),
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
    message: format!("Large event {} with extensive fields", event_id),
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
