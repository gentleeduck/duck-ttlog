// This file shows various ways to use the ttlog library

use std::thread;
use std::time::Duration;
use tracing::{debug, error, info, warn};
use ttlog::trace::Trace;

/// Example 1: Basic logging setup
pub fn example_basic_logging() {
  println!("=== Example 1: Basic Logging ===");

  // Initialize the tracing system
  // - Ring buffer capacity: 1000 events
  // - Channel capacity: 100 pending messages
  let trace_system = Trace::init(1000, 100);

  // Log some events at different levels
  info!("Application started");
  debug!("Debug information");
  warn!("This is a warning");
  error!("An error occurred");

  // Request a manual snapshot
  trace_system.request_snapshot("manual_example");

  // Give time for snapshot to be written
  thread::sleep(Duration::from_millis(200));

  println!("Check /tmp/ for ttlog-*.bin files");
}

// Additional utility functions that might be helpful

/// Helper function to simulate application work with logging
fn simulate_work_with_logging(work_id: u32, duration_ms: u64) {
  info!(work_id = work_id, "Starting work");

  let start = std::time::Instant::now();
  thread::sleep(Duration::from_millis(duration_ms));

  let elapsed = start.elapsed();
  info!(
    work_id = work_id,
    duration_ms = elapsed.as_millis(),
    "Work completed"
  );
}

/// Helper function for batch processing with periodic snapshots
fn batch_process_with_snapshots(trace_system: &Trace, batch_size: usize, total_items: usize) {
  info!("Starting batch processing: {} items", total_items);

  for i in 0..total_items {
    debug!(item = i, "Processing item");

    // Take snapshot at batch boundaries
    if i > 0 && i % batch_size == 0 {
      info!("Completed batch: {} items processed", i);
      trace_system.request_snapshot(format!("batch_checkpoint_{}", i));
    }

    // Simulate processing time
    if i % 100 == 0 {
      thread::sleep(Duration::from_millis(1));
    }
  }

  info!("Batch processing completed: {} total items", total_items);
  trace_system.request_snapshot("batch_final");
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_example_creates_snapshots() {
    let trace_system = Trace::init(100, 10);

    info!("Test log message");
    trace_system.request_snapshot("test_snapshot");

    thread::sleep(Duration::from_millis(100));

    // Check that a snapshot file was created
    let entries: Vec<_> = fs::read_dir("/tmp")
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| e.file_name().to_string_lossy().contains("test_snapshot"))
      .collect();

    assert!(!entries.is_empty(), "Expected snapshot file to be created");
  }
}
