#[cfg(test)]
mod tests {
  use crate::{buffer::RingBuffer, event::Event, panic_hook::PanicHook};

  use std::{
    fs, panic,
    sync::{Arc, Mutex},
  };

  #[test]
  fn test_panic_hook_creates_snapshot_file() {
    let buffer = Arc::new(Mutex::new(RingBuffer::<Event>::new(10)));
    PanicHook::install(buffer.clone());

    let result = panic::catch_unwind(|| {
      panic!("Trigger panic for testing hook");
    });

    assert!(result.is_err(), "Expected a panic");

    // Check /tmp for any file created by flush_snapshot
    let entries: Vec<_> = fs::read_dir("/tmp")
      .unwrap()
      .filter_map(|e| e.ok())
      .filter(|e| e.file_name().to_string_lossy().starts_with("ttlog-"))
      .collect();

    assert!(
      !entries.is_empty(),
      "Expected a snapshot file to be created"
    );
  }
}
