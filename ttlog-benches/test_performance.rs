use std::sync::Arc;
use std::thread;
use std::time::Instant;

use ttlog::{
  event::{FieldValue, LogEvent, LogLevel},
  event_builder::EventBuilder,
  lf_buffer::LockFreeRingBuffer,
  snapshot::SnapshotWriter,
  string_interner::StringInterner,
  trace::Trace,
};

fn main() {
  println!("🚀 TTLog Performance Test Results");
  println!("==================================");
  println!();

  // Test 1: Buffer Operations
  test_buffer_performance();

  // Test 2: Event Creation
  test_event_creation();

  // Test 3: Logging Performance
  test_logging_performance();

  // Test 4: Throughput Limits
  test_throughput_limits();

  // Test 5: Memory Efficiency
  test_memory_efficiency();

  println!("✅ All performance tests completed!");
  println!();
  println!("📊 These numbers show TTLog's raw performance capabilities!");
}

fn test_buffer_performance() {
  println!("🔒 Buffer Operations Performance:");
  println!("--------------------------------");

  // Test push operations
  let buffer = LockFreeRingBuffer::<i32>::new(100000);
  let start = Instant::now();
  for i in 0..100000 {
    buffer.push(i).unwrap();
  }
  let push_time = start.elapsed();
  let push_throughput = 100000.0 / push_time.as_secs_f64();

  println!("  Push 100K items: {:.2} ops/sec", push_throughput);

  // Test pop operations
  let start = Instant::now();
  for _ in 0..100000 {
    let _ = buffer.pop();
  }
  let pop_time = start.elapsed();
  let pop_throughput = 100000.0 / pop_time.as_secs_f64();

  println!("  Pop 100K items:  {:.2} ops/sec", pop_throughput);

  // Test mixed operations
  let buffer = LockFreeRingBuffer::<i32>::new(10000);
  let start = Instant::now();
  for i in 0..10000 {
    buffer.push(i).unwrap();
    let _ = buffer.pop();
  }
  let mixed_time = start.elapsed();
  let mixed_throughput = 20000.0 / mixed_time.as_secs_f64();

  println!("  Mixed push/pop:  {:.2} ops/sec", mixed_throughput);
  println!();
}

fn test_event_creation() {
  println!("📝 Event Creation Performance:");
  println!("------------------------------");

  // Prepare interner and builder
  let interner = Arc::new(StringInterner::new());
  let mut builder = EventBuilder::new(Arc::clone(&interner));

  // Test fast path construction
  let start = Instant::now();
  for i in 0..100000 {
    let _event = builder.build_fast(i as u64, LogLevel::INFO, "test", &format!("Event {}", i));
  }
  let direct_time = start.elapsed();
  let direct_throughput = 100000.0 / direct_time.as_secs_f64();

  println!("  Fast construction:   {:.2} events/sec", direct_throughput);

  // Test build_with_fields with minimal overhead
  let start = Instant::now();
  for i in 0..100000 {
    let _event = builder.build_with_fields(
      i as u64,
      LogLevel::INFO,
      "test",
      &format!("Event {}", i),
      &[],
    );
  }
  let builder_time = start.elapsed();
  let builder_throughput = 100000.0 / builder_time.as_secs_f64();

  println!("  With fields (0):    {:.2} events/sec", builder_throughput);

  // Test with fields
  let start = Instant::now();
  for i in 0..10000 {
    let fields: Vec<(String, FieldValue)> = vec![
      ("user_id".to_string(), FieldValue::I64(i as i64)),
      (
        "action".to_string(),
        FieldValue::StringId(interner.intern_field("login")),
      ),
      ("success".to_string(), FieldValue::Bool(true)),
    ];
    let _event = builder.build_with_fields(
      i as u64,
      LogLevel::INFO,
      "test",
      &format!("Event {}", i),
      &fields,
    );
  }
  let fields_time = start.elapsed();
  let fields_throughput = 10000.0 / fields_time.as_secs_f64();

  println!("  With fields:        {:.2} events/sec", fields_throughput);
  println!();
}

fn test_logging_performance() {
  println!("📊 Logging Performance:");
  println!("-----------------------");

  // Test single-threaded logging
  let _trace_system = Trace::init(100000, 10000);

  let start = Instant::now();
  for i in 0..10000 {
    tracing::info!("Single thread log {}", i);
  }
  let single_time = start.elapsed();
  let single_throughput = 10000.0 / single_time.as_secs_f64();

  println!("  Single thread:      {:.2} logs/sec", single_throughput);

  // Test multi-threaded logging
  let _trace_system = Trace::init(100000, 10000);
  let start = Instant::now();

  let handles: Vec<_> = (0..4)
    .map(|thread_id| {
      thread::spawn(move || {
        for i in 0..2500 {
          tracing::info!("Thread {} log {}", thread_id, i);
        }
      })
    })
    .collect();

  for handle in handles {
    handle.join().unwrap();
  }

  let multi_time = start.elapsed();
  let multi_throughput = 10000.0 / multi_time.as_secs_f64();

  println!("  4 threads:          {:.2} logs/sec", multi_throughput);
  println!();
}

fn test_throughput_limits() {
  println!("🚀 Throughput Limits:");
  println!("---------------------");

  // Test maximum events per second
  let _trace_system = Trace::init(100000, 10000);
  let start = Instant::now();

  for i in 0..50000 {
    tracing::info!("High throughput event {}", i);
  }

  let throughput_time = start.elapsed();
  let throughput_rate = 50000.0 / throughput_time.as_secs_f64();

  println!("  Max events/sec:     {:.2} events/sec", throughput_rate);

  // Test concurrent writers
  let buffer = Arc::new(LockFreeRingBuffer::<i32>::new(100000));
  let start = Instant::now();

  let handles: Vec<_> = (0..8)
    .map(|thread_id| {
      let buffer_clone = Arc::clone(&buffer);
      thread::spawn(move || {
        for i in 0..10000 {
          buffer_clone.push(thread_id * 10000 + i).unwrap();
        }
      })
    })
    .collect();

  for handle in handles {
    handle.join().unwrap();
  }

  let concurrent_time = start.elapsed();
  let concurrent_throughput = 80000.0 / concurrent_time.as_secs_f64();

  println!(
    "  8 concurrent writers: {:.2} ops/sec",
    concurrent_throughput
  );
  println!();
}

fn test_memory_efficiency() {
  println!("💾 Memory Efficiency:");
  println!("---------------------");

  // Test memory per event
  let start = Instant::now();
  let interner = Arc::new(StringInterner::new());
  let mut builder = EventBuilder::new(Arc::clone(&interner));
  let _events: Vec<LogEvent> = (0..10000)
    .map(|i| builder.build_fast(i as u64, LogLevel::INFO, "test", &format!("Event {}", i)))
    .collect();
  let memory_time = start.elapsed();
  let memory_throughput = 10000.0 / memory_time.as_secs_f64();

  println!("  Memory allocation:  {:.2} events/sec", memory_throughput);

  // Test buffer memory efficiency
  let start = Instant::now();
  let buffer = LockFreeRingBuffer::<i32>::new(10000);
  for i in 0..10000 {
    buffer.push(i).unwrap();
  }
  let buffer_time = start.elapsed();
  let buffer_throughput = 10000.0 / buffer_time.as_secs_f64();

  println!("  Buffer operations:  {:.2} ops/sec", buffer_throughput);

  // Test snapshot memory usage
  let start = Instant::now();
  let mut buffer = LockFreeRingBuffer::<LogEvent>::new(1000);
  for i in 0..1000 {
    let event = builder.build_fast(i as u64, LogLevel::INFO, "test", &format!("Event {}", i));
    buffer.push(event).unwrap();
  }

  let writer = SnapshotWriter::new("test-service");
  let _snapshot = writer.create_snapshot(&mut buffer, "performance_test");
  let snapshot_time = start.elapsed();
  let snapshot_throughput = 1000.0 / snapshot_time.as_secs_f64();

  println!(
    "  Snapshot creation:  {:.2} events/sec",
    snapshot_throughput
  );
  println!();
}
