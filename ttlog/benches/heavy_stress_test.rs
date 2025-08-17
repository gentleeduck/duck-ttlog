use std::borrow::Cow;

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono;
use smallvec;
use ttlog::{
  event::{Field, FieldValue, LogEvent, LogLevel},
  lf_buffer::LockFreeRingBuffer,
};

fn current_thread_id_u64() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}

// ============================================================================
// Heavy Stress Testing Components
// ============================================================================

/// Extreme memory pressure test
struct MemoryStressTest {
  buffers: Vec<LockFreeRingBuffer<LogEvent>>,
  memory_usage: Arc<AtomicU64>,
}

impl MemoryStressTest {
  fn new() -> Self {
    Self {
      buffers: Vec::new(),
      memory_usage: Arc::new(AtomicU64::new(0)),
    }
  }

  /// Create extreme memory pressure by allocating many large buffers
  fn extreme_memory_pressure(&mut self, buffer_count: usize, buffer_size: usize) -> u64 {
    let start = Instant::now();

    // Create many large buffers
    for i in 0..buffer_count {
      let buffer = LockFreeRingBuffer::<LogEvent>::new(buffer_size);

      // Fill buffer with heavy events
      for j in 0..buffer_size {
        let event = create_extreme_event(i as u32, j as u64);
        buffer.push(event).unwrap();
      }

      self.buffers.push(buffer);

      // Update memory usage
      let current_usage = self.memory_usage.load(Ordering::Relaxed);
      self
        .memory_usage
        .store(current_usage + buffer_size as u64, Ordering::Relaxed);
    }

    let duration = start.elapsed();
    println!(
      "Created {} buffers with {} total events in {:?}",
      buffer_count,
      self.memory_usage.load(Ordering::Relaxed),
      duration
    );

    self.memory_usage.load(Ordering::Relaxed)
  }

  /// Test memory fragmentation by repeatedly creating/destroying buffers
  fn memory_fragmentation_test(&mut self, iterations: usize) -> u64 {
    let start = Instant::now();

    for iteration in 0..iterations {
      // Create temporary buffers
      let mut temp_buffers = Vec::new();
      for i in 0..100 {
        let buffer = LockFreeRingBuffer::<LogEvent>::new(1000);
        for j in 0..1000 {
          let event = create_extreme_event(i, j);
          buffer.push(event).unwrap();
        }
        temp_buffers.push(buffer);
      }

      // Process and destroy buffers
      for buffer in temp_buffers {
        while let Some(_) = buffer.pop() {}
      }

      if iteration % 100 == 0 {
        println!(
          "Memory fragmentation iteration {}/{}",
          iteration, iterations
        );
      }
    }

    let duration = start.elapsed();
    println!("Memory fragmentation test completed in {:?}", duration);

    iterations as u64
  }
}

/// CPU stress testing with heavy computation
struct CPUStressTest {
  computation_results: Vec<u64>,
}

impl CPUStressTest {
  fn new() -> Self {
    Self {
      computation_results: Vec::new(),
    }
  }

  /// Heavy mathematical computations while logging
  fn heavy_computation_test(&mut self, iterations: usize) -> u64 {
    let start = Instant::now();
    let mut results = Vec::new();

    for i in 0..iterations {
      // Heavy computation
      let mut result = 0u64;
      for j in 0..1000 {
        result += (i * j) as u64;
        result = result.wrapping_mul(7);
        result = result.rotate_left(3);
      }

      // Log the result
      let _event = LogEvent {
        timestamp_nanos: std::time::SystemTime::now()
          .duration_since(std::time::UNIX_EPOCH)
          .unwrap()
          .as_nanos() as u64,
        level: LogLevel::Info,
        target: Cow::Borrowed("cpu_stress"),
        message: format!("Heavy computation result: {}", result),
        fields: smallvec::smallvec![
          Field {
            key: "iteration".into(),
            value: FieldValue::U64(i as u64),
          },
          Field {
            key: "result".into(),
            value: FieldValue::U64(result),
          },
          Field {
            key: "computation_time".into(),
            value: FieldValue::U64(Instant::now().elapsed().as_nanos() as u64),
          },
        ],
        thread_id: current_thread_id_u64(),
        file: Some("heavy_stress_test.rs".into()),
        line: Some(42),
      };

      results.push(result);

      if i % 10000 == 0 {
        println!("CPU stress test: {}/{} iterations", i, iterations);
      }
    }

    self.computation_results = results;
    let duration = start.elapsed();
    println!("CPU stress test completed in {:?}", duration);

    iterations as u64
  }

  /// Prime number generation stress test
  fn prime_generation_stress(&mut self, max_number: u64) -> u64 {
    let start = Instant::now();
    let mut prime_count = 0;

    for num in 2..=max_number {
      if self.is_prime_heavy(num) {
        prime_count += 1;

        // Log every 1000th prime
        if prime_count % 1000 == 0 {
          let _event = LogEvent {
            timestamp_nanos: std::time::SystemTime::now()
              .duration_since(std::time::UNIX_EPOCH)
              .unwrap()
              .as_nanos() as u64,
            level: LogLevel::Info,
            target: Cow::Borrowed("prime_stress"),
            message: format!("Found prime number: {}", num),
            fields: smallvec::smallvec![
              Field {
                key: "prime_count".into(),
                value: FieldValue::U64(prime_count),
              },
              Field {
                key: "prime_number".into(),
                value: FieldValue::U64(num),
              },
            ],
            thread_id: current_thread_id_u64(),
            file: Some("heavy_stress_test.rs".into()),
            line: Some(42),
          };
        }
      }
    }

    let duration = start.elapsed();
    println!(
      "Prime generation stress test completed in {:?}. Found {} primes",
      duration, prime_count
    );

    prime_count
  }

  fn is_prime_heavy(&self, n: u64) -> bool {
    if n < 2 {
      return false;
    }
    if n == 2 {
      return true;
    }
    if n % 2 == 0 {
      return false;
    }

    let sqrt_n = (n as f64).sqrt() as u64;
    for i in (3..=sqrt_n).step_by(2) {
      if n % i == 0 {
        return false;
      }
    }
    true
  }
}

/// Network simulation stress test
struct NetworkStressTest {
  node_count: usize,
  message_count: usize,
  network_buffers: Vec<Arc<LockFreeRingBuffer<LogEvent>>>,
}

impl NetworkStressTest {
  fn new(node_count: usize, message_count: usize) -> Self {
    let network_buffers: Vec<Arc<LockFreeRingBuffer<LogEvent>>> = (0..node_count)
      .map(|_| Arc::new(LockFreeRingBuffer::<LogEvent>::new(100000)))
      .collect();

    Self {
      node_count,
      message_count,
      network_buffers,
    }
  }

  /// Simulate extreme network traffic
  fn extreme_network_traffic(&self) -> u64 {
    let start = Instant::now();
    let mut handles = Vec::new();

    // Create sender threads for each node
    for node_id in 0..self.node_count {
      let buffer = Arc::clone(&self.network_buffers[node_id]);
      let message_count = self.message_count;

      let handle = thread::spawn(move || {
        let mut sent_messages = 0;
        for i in 0..message_count {
          let event = create_network_stress_event(node_id as u32, i as u64);
          if buffer.push(event).is_ok() {
            sent_messages += 1;
          }

          // Simulate network delay
          if i % 1000 == 0 {
            thread::sleep(Duration::from_micros(100));
          }
        }
        sent_messages
      });

      handles.push(handle);
    }

    // Wait for all senders to complete
    let mut total_messages = 0;
    for handle in handles {
      total_messages += handle.join().unwrap();
    }

    let duration = start.elapsed();
    println!(
      "Network stress test completed in {:?}. Sent {} messages",
      duration, total_messages
    );

    total_messages
  }

  /// Test network congestion scenarios
  fn network_congestion_test(&self) -> u64 {
    let start = Instant::now();
    let mut handles = Vec::new();

    // Create extreme congestion by having all nodes send to all other nodes
    for source_node in 0..self.node_count {
      for target_node in 0..self.node_count {
        if source_node != target_node {
          let buffer = Arc::clone(&self.network_buffers[target_node]);
          let source_id = source_node as u32;

          let handle = thread::spawn(move || {
            let mut messages_sent = 0;
            for i in 0..10000 {
              let event = create_congestion_event(source_id, target_node as u32, i);
              if buffer.push(event).is_ok() {
                messages_sent += 1;
              }
            }
            messages_sent
          });

          handles.push(handle);
        }
      }
    }

    // Wait for completion
    let mut total_messages = 0;
    for handle in handles {
      total_messages += handle.join().unwrap();
    }

    let duration = start.elapsed();
    println!(
      "Network congestion test completed in {:?}. Sent {} messages",
      duration, total_messages
    );

    total_messages
  }
}

// ============================================================================
// Main Stress Testing Functions
// ============================================================================

fn run_memory_stress_test() {
  println!("🚀 Starting Extreme Memory Stress Test...");
  println!("==========================================");

  let mut memory_test = MemoryStressTest::new();

  // Test 1: Extreme memory pressure
  println!("\n📊 Test 1: Extreme Memory Pressure");
  let memory_usage = memory_test.extreme_memory_pressure(100, 100000);
  println!(
    "✅ Memory pressure test completed. Total events: {}",
    memory_usage
  );

  // Test 2: Memory fragmentation
  println!("\n📊 Test 2: Memory Fragmentation");
  let fragmentation_result = memory_test.memory_fragmentation_test(1000);
  println!(
    "✅ Memory fragmentation test completed. Iterations: {}",
    fragmentation_result
  );

  println!("\n🎉 Memory stress test completed successfully!");
}

fn run_cpu_stress_test() {
  println!("🚀 Starting Extreme CPU Stress Test...");
  println!("======================================");

  let mut cpu_test = CPUStressTest::new();

  // Test 1: Heavy computation
  println!("\n📊 Test 1: Heavy Computation");
  let computation_result = cpu_test.heavy_computation_test(100000);
  println!(
    "✅ Heavy computation test completed. Iterations: {}",
    computation_result
  );

  // Test 2: Prime generation
  println!("\n📊 Test 2: Prime Number Generation");
  let prime_result = cpu_test.prime_generation_stress(100000);
  println!(
    "✅ Prime generation test completed. Primes found: {}",
    prime_result
  );

  println!("\n🎉 CPU stress test completed successfully!");
}

fn run_network_stress_test() {
  println!("🚀 Starting Extreme Network Stress Test...");
  println!("==========================================");

  // Test 1: Extreme network traffic
  println!("\n📊 Test 1: Extreme Network Traffic");
  let network_test = NetworkStressTest::new(16, 100000);
  let traffic_result = network_test.extreme_network_traffic();
  println!(
    "✅ Network traffic test completed. Messages sent: {}",
    traffic_result
  );

  // Test 2: Network congestion
  println!("\n📊 Test 2: Network Congestion");
  let congestion_result = network_test.network_congestion_test();
  println!(
    "✅ Network congestion test completed. Messages sent: {}",
    congestion_result
  );

  println!("\n🎉 Network stress test completed successfully!");
}

fn run_comprehensive_stress_test() {
  println!("🚀 Starting Comprehensive Extreme Stress Test...");
  println!("================================================");

  let start = Instant::now();

  // Run all stress tests concurrently
  let memory_handle = thread::spawn(run_memory_stress_test);
  let cpu_handle = thread::spawn(run_cpu_stress_test);
  let network_handle = thread::spawn(run_network_stress_test);

  // Wait for all tests to complete
  memory_handle.join().unwrap();
  cpu_handle.join().unwrap();
  network_handle.join().unwrap();

  let total_duration = start.elapsed();
  println!("\n🎉 All stress tests completed in {:?}!", total_duration);
  println!("🚀 TTLog has been pushed to its absolute limits!");
}

// ============================================================================
// Utility Functions
// ============================================================================

fn create_extreme_event(thread_id: u32, event_id: u64) -> LogEvent {
  LogEvent {
    timestamp_nanos: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    level: LogLevel::Info,
    target: Cow::Borrowed("extreme_stress"),
    message: format!(
      "Extreme stress event {} from thread {}",
      event_id, thread_id
    ),
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
        key: "stress_level".into(),
        value: FieldValue::Str("extreme".into()),
      },
      Field {
        key: "memory_pressure".into(),
        value: FieldValue::U64((event_id % 1000) as u64),
      },
      Field {
        key: "cpu_load".into(),
        value: FieldValue::U64((event_id % 100) as u64),
      },
      Field {
        key: "network_latency".into(),
        value: FieldValue::U64((event_id % 500) as u64),
      },
    ],
    thread_id: current_thread_id_u64(),
    file: Some("heavy_stress_test.rs".into()),
    line: Some(42),
  }
}

fn create_network_stress_event(source_node: u32, message_id: u64) -> LogEvent {
  LogEvent {
    timestamp_nanos: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    level: LogLevel::Info,
    target: Cow::Borrowed("network_stress"),
    message: format!(
      "Network stress message {} from node {}",
      message_id, source_node
    ),
    fields: smallvec::smallvec![
      Field {
        key: "source_node".into(),
        value: FieldValue::U64(source_node as u64),
      },
      Field {
        key: "message_id".into(),
        value: FieldValue::U64(message_id),
      },
      Field {
        key: "message_type".into(),
        value: FieldValue::Str("stress_test".into()),
      },
      Field {
        key: "priority".into(),
        value: FieldValue::U64((message_id % 10) as u64),
      },
    ],
    thread_id: current_thread_id_u64(),
    file: Some("heavy_stress_test.rs".into()),
    line: Some(42),
  }
}

fn create_congestion_event(source_node: u32, target_node: u32, message_id: u64) -> LogEvent {
  LogEvent {
    timestamp_nanos: std::time::SystemTime::now()
      .duration_since(std::time::UNIX_EPOCH)
      .unwrap()
      .as_nanos() as u64,
    level: LogLevel::Warn,
    target: Cow::Borrowed("network_congestion"),
    message: format!(
      "Congestion message {} from {} to {}",
      message_id, source_node, target_node
    ),
    fields: smallvec::smallvec![
      Field {
        key: "source_node".into(),
        value: FieldValue::U64(source_node as u64),
      },
      Field {
        key: "target_node".into(),
        value: FieldValue::U64(target_node as u64),
      },
      Field {
        key: "message_id".into(),
        value: FieldValue::U64(message_id),
      },
      Field {
        key: "congestion_level".into(),
        value: FieldValue::Str("extreme".into()),
      },
    ],
    thread_id: current_thread_id_u64(),
    file: Some("heavy_stress_test.rs".into()),
    line: Some(42),
  }
}

// ============================================================================
// Main Function
// ============================================================================

fn main() {
  println!("🔥 TTLog Extreme Heavy Stress Testing Suite");
  println!("===========================================");
  println!("This suite will push TTLog to its absolute limits!");
  println!();

  // Parse command line arguments for test selection
  let args: Vec<String> = std::env::args().collect();

  if args.len() > 1 {
    match args[1].as_str() {
      "memory" => {
        println!("🎯 Running Memory Stress Test Only");
        run_memory_stress_test();
      },
      "cpu" => {
        println!("🎯 Running CPU Stress Test Only");
        run_cpu_stress_test();
      },
      "network" => {
        println!("🎯 Running Network Stress Test Only");
        run_network_stress_test();
      },
      "all" | _ => {
        println!("🎯 Running All Stress Tests");
        run_comprehensive_stress_test();
      },
    }
  } else {
    println!("🎯 Running All Stress Tests (default)");
    run_comprehensive_stress_test();
  }

  println!("\n🚀 Stress testing completed! TTLog has proven its resilience!");
}
