use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::{Duration, Instant};

use chrono;
use serde_cbor;
use serde_json;
use smallvec; // kept for any non-event usage; no longer used for fields
use ttlog::{
  event::{FieldValue, LogEvent, LogLevel},
  event_builder::EventBuilder,
  lf_buffer::LockFreeRingBuffer,
  snapshot::SnapshotWriter,
  string_interner::StringInterner,
};

// Shared interner and thread-local builder for fast event creation
static INTERNER: std::sync::OnceLock<Arc<StringInterner>> = std::sync::OnceLock::new();
thread_local! { static BUILDER: std::cell::RefCell<Option<EventBuilder>> = std::cell::RefCell::new(None); }

// Configure Criterion for reliable benchmarks
fn configure_criterion() -> Criterion {
  Criterion::default()
    .sample_size(30) // More samples for reliability
    .measurement_time(Duration::from_secs(10)) // Longer measurement time
    .warm_up_time(Duration::from_secs(5)) // Proper warmup
    .confidence_level(0.95) // 95% confidence interval
    .significance_level(0.05) // 5% significance level
    .noise_threshold(0.05) // 5% noise threshold
}

fn current_thread_id_u64() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}

// ============================================================================
// Distributed System Simulation Components
// ============================================================================

/// Simulates a distributed node with multiple worker threads
struct DistributedNode {
  node_id: u32,
  workers: Vec<thread::JoinHandle<()>>,
  event_count: Arc<AtomicU64>,
  buffer: Arc<LockFreeRingBuffer<LogEvent>>,
}

impl DistributedNode {
  fn new(node_id: u32, worker_count: usize, buffer_size: usize) -> Self {
    let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(buffer_size));
    let event_count = Arc::new(AtomicU64::new(0));

    let workers: Vec<thread::JoinHandle<()>> = (0..worker_count)
      .map(|worker_id| {
        let buffer_clone = Arc::clone(&buffer);
        let event_count_clone = Arc::clone(&event_count);
        let node_id = node_id;

        thread::spawn(move || {
          for i in 0..1000 {
            let interner = INTERNER
              .get_or_init(|| Arc::new(StringInterner::new()))
              .clone();
            let event = BUILDER.with(|cell| {
              if cell.borrow().is_none() {
                *cell.borrow_mut() = Some(EventBuilder::new(interner.clone()));
              }
              let mut builder_ref = cell.borrow_mut();
              let builder = builder_ref.as_mut().unwrap();
              let ts = chrono::Utc::now().timestamp_millis() as u64;
              let target = format!("node_{}_worker_{}", node_id, worker_id);
              let msg = format!("Distributed event {} from worker {}", i, worker_id);
              let fields: Vec<(String, FieldValue)> = vec![
                ("node_id".to_string(), FieldValue::U64(node_id as u64)),
                ("worker_id".to_string(), FieldValue::U64(worker_id as u64)),
                ("event_id".to_string(), FieldValue::U64(i as u64)),
              ];
              builder.build_with_fields(ts, LogLevel::INFO, &target, &msg, &fields)
            });

            // Handle buffer full condition gracefully
            if let Err(_) = buffer_clone.push(event) {
              // Buffer is full, skip this event or wait
              thread::sleep(Duration::from_micros(100));
              continue;
            }
            event_count_clone.fetch_add(1, Ordering::Relaxed);

            // Simulate some work
            thread::sleep(Duration::from_micros(10));
          }
        })
      })
      .collect();

    Self {
      node_id,
      workers,
      event_count,
      buffer,
    }
  }

  fn wait_for_completion(self) -> u64 {
    for worker in self.workers {
      worker.join().unwrap();
    }
    self.event_count.load(Ordering::Relaxed)
  }

  fn get_buffer(self) -> LockFreeRingBuffer<LogEvent> {
    Arc::try_unwrap(self.buffer).unwrap()
  }
}

/// Simulates network communication between nodes
struct NetworkSimulator {
  nodes: Vec<DistributedNode>,
  network_buffer: Arc<LockFreeRingBuffer<LogEvent>>,
  network_latency: Duration,
}

impl NetworkSimulator {
  fn new(node_count: usize, workers_per_node: usize, buffer_size: usize, latency_ms: u64) -> Self {
    let nodes: Vec<DistributedNode> = (0..node_count)
      .map(|node_id| DistributedNode::new(node_id as u32, workers_per_node, buffer_size))
      .collect();

    let network_buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(
      buffer_size * node_count,
    ));
    let network_latency = Duration::from_millis(latency_ms);

    Self {
      nodes,
      network_buffer,
      network_latency,
    }
  }

  fn simulate_network_communication(&self) -> u64 {
    let total_events = 0;

    // Simulate network communication with latency
    for _node in &self.nodes {
      let node_buffer = Arc::clone(&self.network_buffer);
      let latency = self.network_latency;

      thread::spawn(move || {
        thread::sleep(latency);
        // Simulate network transfer
        for _ in 0..1000 {
          let event = create_network_event();
          // Handle buffer full condition gracefully
          if let Err(_) = node_buffer.push(event) {
            // Buffer is full, skip this event
            continue;
          }
        }
      });
    }

    total_events
  }

  fn run_distributed_workload(&self, duration: Duration) -> u64 {
    let start = Instant::now();
    let mut total_events = 0;

    while start.elapsed() < duration {
      for _node in &self.nodes {
        total_events += _node.event_count.load(Ordering::Relaxed);
      }
      thread::sleep(Duration::from_millis(100));
    }

    total_events
  }
}

// ============================================================================
// Maximum-Level Heavy Benchmarks
// ============================================================================

fn bench_distributed_node_performance(c: &mut Criterion) {
  let mut group = c.benchmark_group("distributed_node_performance");

  // Test single node with varying worker counts
  for worker_count in [1, 4, 8, 16, 32].iter() {
    group.bench_with_input(
      BenchmarkId::new("workers", worker_count),
      worker_count,
      |b, &worker_count| {
        b.iter(|| {
          let node = DistributedNode::new(1, worker_count, 100000);
          let event_count = node.wait_for_completion();
          event_count
        });
      },
    );
  }

  // Test node with varying buffer sizes
  for buffer_size in [100000, 500000, 1000000, 5000000].iter() {
    group.bench_with_input(
      BenchmarkId::new("buffer_size", buffer_size),
      buffer_size,
      |b, &buffer_size| {
        b.iter(|| {
          let node = DistributedNode::new(1, 8, buffer_size);
          let event_count = node.wait_for_completion();
          event_count
        });
      },
    );
  }

  group.finish();
}

fn bench_multi_node_cluster(c: &mut Criterion) {
  let mut group = c.benchmark_group("multi_node_cluster");

  // Test cluster with varying node counts
  for node_count in [2, 4, 8, 16].iter() {
    group.bench_with_input(
      BenchmarkId::new("nodes", node_count),
      node_count,
      |b, &node_count| {
        b.iter(|| {
          let simulator = NetworkSimulator::new(node_count, 4, 50000, 5);
          let total_events = simulator.run_distributed_workload(Duration::from_secs(1));
          total_events
        });
      },
    );
  }

  // Test cluster with varying network latencies
  for latency_ms in [1, 5, 10, 50, 100].iter() {
    group.bench_with_input(
      BenchmarkId::new("latency_ms", latency_ms),
      latency_ms,
      |b, &latency_ms| {
        b.iter(|| {
          let simulator = NetworkSimulator::new(4, 4, 50000, latency_ms);
          simulator.simulate_network_communication()
        });
      },
    );
  }

  group.finish();
}

fn bench_extreme_concurrency(c: &mut Criterion) {
  let mut group = c.benchmark_group("extreme_concurrency");

  // Test extreme thread counts
  for thread_count in [32, 64, 128, 256].iter() {
    group.bench_with_input(
      BenchmarkId::new("threads", thread_count),
      thread_count,
      |b, &thread_count| {
        b.iter(|| {
          let buffer = Arc::new(LockFreeRingBuffer::<LogEvent>::new(1000000));
          let handles: Vec<_> = (0..thread_count)
            .map(|thread_id| {
              let buffer_clone = Arc::clone(&buffer);
              thread::spawn(move || {
                for i in 0..1000 {
                  let event = create_heavy_event(thread_id, i);
                  buffer_clone.push(event).unwrap();
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }

          buffer.len()
        });
      },
    );
  }

  // Test extreme buffer operations
  group.bench_function("extreme_buffer_operations", |b| {
    b.iter(|| {
      let buffer = LockFreeRingBuffer::<LogEvent>::new(1000000);

      // Fill buffer completely
      for i in 0..1000000 {
        let event = create_heavy_event(0, i);
        if let Err(_) = buffer.push(event) {
          break; // Buffer is full
        }
      }

      // Drain buffer completely
      let mut drained = 0;
      while let Some(_) = buffer.pop() {
        drained += 1;
      }
      drained;

      buffer.len()
    });
  });

  group.finish();
}

fn bench_memory_stress_testing(c: &mut Criterion) {
  let mut group = c.benchmark_group("memory_stress_testing");

  // Test memory pressure with large event counts
  for event_count in [10000, 100000, 1000000].iter() {
    group.bench_with_input(
      BenchmarkId::new("events", event_count),
      event_count,
      |b, &event_count| {
        b.iter(|| {
          let mut buffers = Vec::new();

          // Create multiple buffers to simulate memory pressure
          for i in 0..100 {
            let buffer = LockFreeRingBuffer::<LogEvent>::new(event_count / 100);
            for j in 0..(event_count / 100) {
              let event = create_heavy_event(i, j as u64);
              if let Err(_) = buffer.push(event) {
                break; // Buffer is full
              }
            }
            buffers.push(buffer);
          }

          // Process all buffers
          let mut total_events = 0;
          for buffer in buffers {
            while let Some(_) = buffer.pop() {
              total_events += 1;
            }
          }

          total_events
        });
      },
    );
  }

  // Test memory fragmentation
  group.bench_function("memory_fragmentation", |b| {
    b.iter(|| {
      let mut buffers = Vec::new();

      // Create and destroy buffers repeatedly to cause fragmentation
      for _ in 0..1000 {
        let buffer = LockFreeRingBuffer::<LogEvent>::new(1000);
        for i in 0..1000 {
          let event = create_heavy_event(0, i);
          if let Err(_) = buffer.push(event) {
            break; // Buffer is full
          }
        }

        // Drain buffer
        while let Some(_) = buffer.pop() {}

        buffers.push(buffer);

        // Keep only last 100 buffers to prevent memory explosion
        if buffers.len() > 100 {
          buffers.remove(0);
        }
      }

      buffers.len()
    });
  });

  group.finish();
}

fn bench_network_simulation(c: &mut Criterion) {
  let mut group = c.benchmark_group("network_simulation");

  // Test network throughput simulation
  group.bench_function("network_throughput", |b| {
    b.iter(|| {
      let simulator = NetworkSimulator::new(8, 4, 100000, 1);
      let start = Instant::now();

      // Simulate network traffic for 1 second
      let total_events = simulator.run_distributed_workload(Duration::from_secs(1));
      let duration = start.elapsed();

      // Calculate events per second
      total_events as f64 / duration.as_secs_f64()
    });
  });

  // Test network latency simulation
  group.bench_function("network_latency_simulation", |b| {
    b.iter(|| {
      let simulator = NetworkSimulator::new(4, 2, 50000, 10);
      let start = Instant::now();

      simulator.simulate_network_communication();

      start.elapsed().as_millis()
    });
  });

  // Test network congestion
  group.bench_function("network_congestion", |b| {
    b.iter(|| {
      let mut total_events = 0;
      let mut handles = Vec::new();

      // Create many nodes to simulate network congestion
      for node_id in 0..32 {
        let handle = thread::spawn(move || {
          let node = DistributedNode::new(node_id, 2, 50000);
          let event_count = node.wait_for_completion();
          event_count
        });
        handles.push(handle);
      }

      for handle in handles {
        total_events += handle.join().unwrap();
      }

      total_events
    });
  });

  group.finish();
}

fn bench_distributed_snapshot_performance(c: &mut Criterion) {
  let mut group = c.benchmark_group("distributed_snapshot_performance");

  // Test snapshot creation from distributed buffers
  for buffer_size in [100000, 500000, 1000000].iter() {
    group.bench_with_input(
      BenchmarkId::new("buffer_size", buffer_size),
      buffer_size,
      |b, &buffer_size| {
        b.iter(|| {
          let mut buffers = Vec::new();
          let writer = SnapshotWriter::new("distributed-bench");

          // Create multiple buffers with events
          for i in 0..10 {
            let buffer = LockFreeRingBuffer::<LogEvent>::new(buffer_size / 10);
            for j in 0..(buffer_size / 10) {
              let event = create_heavy_event(i, j as u64);
              if let Err(_) = buffer.push(event) {
                break; // Buffer is full
              }
            }
            buffers.push(buffer);
          }

          // Create snapshots from all buffers
          let mut total_snapshots = 0;
          for buffer in &mut buffers {
            if let Some(_) = writer.create_snapshot(buffer, "distributed_test") {
              total_snapshots += 1;
            }
          }

          total_snapshots
        });
      },
    );
  }

  // Test concurrent snapshot creation
  group.bench_function("concurrent_snapshots", |b| {
    b.iter(|| {
      let writer = SnapshotWriter::new("concurrent-bench");
      let mut handles = Vec::new();

      // Create multiple snapshot creation threads
      for i in 0..8 {
        let writer_clone = writer.clone();
        let handle = thread::spawn(move || {
          let mut buffer = LockFreeRingBuffer::<LogEvent>::new(10000);
          for j in 0..10000 {
            let event = create_heavy_event(i, j);
            if let Err(_) = buffer.push(event) {
              break; // Buffer is full
            }
          }

          if let Some(_) = writer_clone.create_snapshot(&mut buffer, "concurrent_test") {
            1
          } else {
            0
          }
        });
        handles.push(handle);
      }

      let mut total_snapshots = 0;
      for handle in handles {
        total_snapshots += handle.join().unwrap();
      }

      total_snapshots
    });
  });

  group.finish();
}

fn bench_extreme_serialization(c: &mut Criterion) {
  let mut group = c.benchmark_group("extreme_serialization");

  // Test CBOR serialization with large events
  for event_count in [1000, 10000, 100000].iter() {
    group.bench_with_input(
      BenchmarkId::new("cborevents", event_count),
      event_count,
      |b, &event_count| {
        b.iter(|| {
          let events: Vec<LogEvent> = (0..event_count).map(|i| create_heavy_event(0, i)).collect();

          let snapshot = ttlog::snapshot::Snapshot {
            service: "extreme-bench".to_string(),
            hostname: "test-host".to_string(),
            pid: std::process::id(),
            created_at: "20240101120000".to_string(),
            reason: "extreme_serialization_test".to_string(),
            events,
          };

          let serialized = serde_cbor::to_vec(&snapshot).unwrap();
          serialized.len()
        });
      },
    );
  }

  // Test JSON serialization with large events
  for event_count in [1000, 10000, 100000].iter() {
    group.bench_with_input(
      BenchmarkId::new("jsonevents", event_count),
      event_count,
      |b, &event_count| {
        b.iter(|| {
          let events: Vec<LogEvent> = (0..event_count).map(|i| create_heavy_event(0, i)).collect();

          let snapshot = ttlog::snapshot::Snapshot {
            service: "extreme-bench".to_string(),
            hostname: "test-host".to_string(),
            pid: std::process::id(),
            created_at: "20240101120000".to_string(),
            reason: "extreme_serialization_test".to_string(),
            events,
          };

          let serialized = serde_json::to_string(&snapshot).unwrap();
          serialized.len()
        });
      },
    );
  }

  group.finish();
}

// ============================================================================
// Utility Functions
// ============================================================================

fn create_heavy_event(thread_id: u32, event_id: u64) -> LogEvent {
  let interner = INTERNER
    .get_or_init(|| Arc::new(StringInterner::new()))
    .clone();
  BUILDER.with(|cell| {
    if cell.borrow().is_none() {
      *cell.borrow_mut() = Some(EventBuilder::new(interner.clone()));
    }
    let mut builder_ref = cell.borrow_mut();
    let builder = builder_ref.as_mut().unwrap();
    let ts = chrono::Utc::now().timestamp_millis() as u64;
    let msg = format!(
      "Heavy distributed event {} from thread {}",
      event_id, thread_id
    );
    let fields: Vec<(String, FieldValue)> = vec![
      ("thread_id".to_string(), FieldValue::U64(thread_id as u64)),
      ("event_id".to_string(), FieldValue::U64(event_id)),
      (
        "category".to_string(),
        FieldValue::StringId(interner.intern_field("distributed_system")),
      ),
    ];
    builder.build_with_fields(ts, LogLevel::INFO, "extreme_bench", &msg, &fields)
  })
}

fn create_network_event() -> LogEvent {
  let interner = INTERNER
    .get_or_init(|| Arc::new(StringInterner::new()))
    .clone();
  BUILDER.with(|cell| {
    if cell.borrow().is_none() {
      *cell.borrow_mut() = Some(EventBuilder::new(interner.clone()));
    }
    let mut builder_ref = cell.borrow_mut();
    let builder = builder_ref.as_mut().unwrap();
    let ts = chrono::Utc::now().timestamp_millis() as u64;
    let fields: Vec<(String, FieldValue)> = vec![
      ("source_node".to_string(), FieldValue::U64(1)),
      ("target_node".to_string(), FieldValue::U64(2)),
      (
        "message_type".to_string(),
        FieldValue::StringId(interner.intern_field("heartbeat")),
      ),
    ];
    builder.build_with_fields(
      ts,
      LogLevel::INFO,
      "network_sim",
      "Network communication event",
      &fields,
    )
  })
}

// ============================================================================
// Benchmark Groups
// ============================================================================

criterion_group! {
  name = benches;
  config = configure_criterion();
  targets =
    bench_distributed_node_performance,
    bench_multi_node_cluster,
    bench_extreme_concurrency,
    bench_memory_stress_testing,
    bench_network_simulation,
    bench_distributed_snapshot_performance,
    bench_extreme_serialization,
}

criterion_main!(benches);
