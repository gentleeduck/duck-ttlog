use std::thread;
use std::time::Duration;
use tracing::{error, info, warn};
use ttlog::{panic_hook::PanicHook, trace::Trace};

/// Example 5: Panic handling with automatic snapshots
pub fn example_panic_handling() {
  println!("\n=== Example 5: Panic Handling ===");

  let trace_system = Trace::init(500, 50);

  // Install panic hook to capture snapshots on panics
  PanicHook::install(trace_system.get_sender());

  // Log some events
  info!("About to do some risky operations");
  warn!("This might cause issues");

  // Simulate a controlled panic (in real code, this would be an unexpected panic)
  let result = std::panic::catch_unwind(|| {
    error!("Something went wrong before panic");
    panic!("Simulated application crash!");
  });

  match result {
    Ok(_) => println!("No panic occurred"),
    Err(_) => {
      println!("Panic was caught! Check /tmp/ for panic snapshot");
      // Give extra time for panic snapshot to be written
      thread::sleep(Duration::from_millis(1000));
    },
  }
}
