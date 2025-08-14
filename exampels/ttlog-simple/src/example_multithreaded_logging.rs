use std::thread;
use std::time::Duration;
use tracing::{info, warn};
use ttlog::trace::Trace;

/// Example 4: Multi-threaded logging
pub fn example_multithreaded_logging() {
  println!("\n=== Example 4: Multi-threaded Logging ===");

  let trace_system = Trace::init(1000, 150);

  let handles: Vec<_> = (0..5)
    .map(|thread_id| {
      thread::spawn(move || {
        for i in 0..100 {
          info!(
            thread_id = thread_id,
            iteration = i,
            "Thread {} processing item {}",
            thread_id,
            i
          );

          if i % 50 == 0 {
            warn!(thread_id = thread_id, "Halfway point reached");
          }

          // Small delay to simulate work
          thread::sleep(Duration::from_millis(1));
        }

        info!(thread_id = thread_id, "Thread completed");
      })
    })
    .collect();

  // Wait for all threads to complete
  for handle in handles {
    handle.join().unwrap();
  }

  // Take final snapshot
  trace_system.request_snapshot("multithreaded_complete");
  thread::sleep(Duration::from_millis(200));
}
