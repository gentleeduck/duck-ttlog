mod example_basic_logging;
mod example_custom_service;
mod example_error_scenarios;
mod example_high_volume_logging;
mod example_multithreaded_logging;
mod example_panic_handling;
mod example_simple;
mod example_structured_logging;

use std::{thread, time::Duration};

use crate::{
  example_basic_logging::example_basic_logging, example_custom_service::example_custom_service,
  example_error_scenarios::example_error_scenarios,
  example_high_volume_logging::example_high_volume_logging,
  example_multithreaded_logging::example_multithreaded_logging,
  example_panic_handling::example_panic_handling, example_simple::example_simple,
  example_structured_logging::example_structured_logging,
};

fn main() {
  println!("TTLog Library Examples");
  println!("=====================");

  // Run all examples
  example_simple();
  // example_basic_logging();
  // example_structured_logging();
  // example_high_volume_logging();
  // example_multithreaded_logging();
  // example_panic_handling();
  // example_custom_service();
  // example_error_scenarios();

  // println!("\n=== All Examples Completed ===");
  // println!("Check /tmp/ directory for generated snapshot files:");
  // println!("  ls -la /tmp/ttlog-*.bin");
  // println!("\nTo decompress and view a snapshot file:");
  // println!("  # This would require a separate utility to decompress LZ4 and decode CBOR");
  //
  // // Give final time for all snapshots to be written
  // thread::sleep(Duration::from_millis(500));
}
