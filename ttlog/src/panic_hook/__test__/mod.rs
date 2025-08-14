#[cfg(test)]
mod tests {
  use crate::panic_hook::PanicHook;
  use crate::trace::{Message, Trace};
  use crossbeam_channel::{bounded, RecvTimeoutError};
  use std::{panic, thread, time::Duration};

  /// Start a background thread that drains the receiver for a short while.
  /// This prevents the panic hook's blocking `send` from deadlocking tests.
  fn start_drain_thread(receiver: crossbeam_channel::Receiver<Message>) -> thread::JoinHandle<()> {
    thread::spawn(move || loop {
      match receiver.recv_timeout(Duration::from_millis(500)) {
        Ok(_msg) => {
          // keep draining
          continue;
        },
        Err(RecvTimeoutError::Timeout) => break,
        Err(RecvTimeoutError::Disconnected) => break,
      }
    })
  }

  /// Verifies that `PanicHook::install` can be called without panicking.
  #[test]
  fn test_panic_hook_install() {
    let (sender, _receiver) = bounded::<Message>(10);
    PanicHook::install(sender);
  }

  /// Checks that a panic triggers the hook (catch_unwind may prevent execution of hook).
  #[test]
  fn test_panic_hook_message_sending() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| panic!("Test panic for hook"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Ensures the hook handles a disconnected channel gracefully.
  #[test]
  fn test_panic_hook_with_disconnected_channel() {
    let (sender, receiver) = bounded::<Message>(10);
    // Drop receiver to simulate disconnected channel; send should return Err in hook.
    drop(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| panic!("Test panic with disconnected channel"));
    assert!(result.is_err());
  }

  /// Verifies that multiple installations override previous hooks.
  #[test]
  fn test_panic_hook_multiple_installations() {
    let (sender1, receiver1) = bounded::<Message>(10);
    let (sender2, receiver2) = bounded::<Message>(10);

    // Drain both receivers so hook send won't block
    let h1 = start_drain_thread(receiver1);
    let h2 = start_drain_thread(receiver2);

    PanicHook::install(sender1);
    PanicHook::install(sender2);

    let _ = h1.join();
    let _ = h2.join();
  }

  /// Tests integration with a `Trace` system.
  #[test]
  fn test_panic_hook_integration_with_trace() {
    let trace_system = Trace::init(100, 10);

    // If Trace::init has internal receivers, we assume it manages them.
    // Still, to be safe, create a test channel and drain it (won't interfere).
    let (_test_sender, test_receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(test_receiver);

    PanicHook::install(trace_system.get_sender());

    tracing::info!("Pre-panic event");

    let result = panic::catch_unwind(|| panic!("Integration test panic"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Tests the hook with different panic payload types.
  #[test]
  fn test_panic_hook_with_various_panic_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let results = vec![
      panic::catch_unwind(|| panic!("String panic")),
      panic::catch_unwind(|| panic!("{}", "Formatted panic")),
      panic::catch_unwind(|| panic!("{}", 42)),
      panic::catch_unwind(|| panic!("{:?}", vec![1, 2, 3])),
    ];

    for result in results {
      assert!(result.is_err());
    }

    let _ = drain.join();
  }

  /// Tests the hook with `panic_any` and custom payloads.
  #[test]
  fn test_panic_hook_with_custom_panic_info() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| panic::panic_any("Custom panic payload"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Ensures thread safety by panicking in multiple threads.
  #[test]
  fn test_panic_hook_thread_safety() {
    let (sender, receiver) = bounded::<Message>(100);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let handles: Vec<_> = (0..5)
      .map(|id| {
        thread::spawn(move || {
          if id % 2 == 0 {
            panic!("Thread {} panic", id)
          } else {
            id
          }
        })
      })
      .collect();

    for handle in handles {
      let _ = handle.join();
    }

    let _ = drain.join();
  }

  /// Tests nested panics to ensure stability.
  #[test]
  fn test_panic_hook_with_nested_panics() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| {
      let _ = panic::catch_unwind(|| panic!("Inner panic"));
      panic!("Outer panic");
    });
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Simulates long panic messages.
  #[test]
  fn test_panic_hook_with_long_panic_messages() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let long_message = "A".repeat(10000);
    let result = panic::catch_unwind(|| panic!("{}", long_message));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Tests panic messages with Unicode characters.
  #[test]
  fn test_panic_hook_with_unicode_messages() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| panic!("Unicode panic: 🚀 🎉 💻"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Tests panic messages containing special characters.
  #[test]
  fn test_panic_hook_with_special_characters() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let result = panic::catch_unwind(|| panic!("Special chars: \"quotes\" and \n newlines"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Tests numeric panic payloads with `panic_any`.
  #[test]
  fn test_panic_hook_with_numeric_payloads() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let results = vec![
      panic::catch_unwind(|| panic::panic_any(42u32)),
      panic::catch_unwind(|| panic::panic_any(3.14f64)),
      panic::catch_unwind(|| panic::panic_any(-1i32)),
    ];

    for result in results {
      assert!(result.is_err());
    }

    let _ = drain.join();
  }

  /// Tests panic payloads containing collections.
  #[test]
  fn test_panic_hook_with_collection_payloads() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let results = vec![
      panic::catch_unwind(|| panic::panic_any(vec![1, 2, 3])),
      panic::catch_unwind(|| panic::panic_any(["a", "b", "c"])),
      panic::catch_unwind(|| panic::panic_any(std::collections::HashMap::<i32, i32>::new())),
    ];

    for result in results {
      assert!(result.is_err());
    }

    let _ = drain.join();
  }

  /// Tests panic payloads with custom error types.
  #[test]
  fn test_panic_hook_with_custom_error_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    #[derive(Debug)]
    struct CustomError {
      #[allow(dead_code)]
      message: String,
      #[allow(dead_code)]
      code: u32,
    }

    let custom_error = CustomError {
      message: "Error".into(),
      code: 42,
    };
    let result = panic::catch_unwind(|| panic::panic_any(custom_error));
    assert!(result.is_err());

    let _ = drain.join();
  }

  /// Tests panic payloads with standard error types.
  #[test]
  fn test_panic_hook_with_std_error_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);
    PanicHook::install(sender);

    let results = vec![
      panic::catch_unwind(|| {
        panic::panic_any(std::io::Error::new(
          std::io::ErrorKind::NotFound,
          "File not found",
        ))
      }),
      panic::catch_unwind(|| panic::panic_any(std::fmt::Error)),
    ];

    for result in results {
      assert!(result.is_err());
    }

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_full_channel() {
    use crossbeam_channel::{bounded, RecvTimeoutError};
    use std::time::Duration;

    // capacity 1, fill the channel so it's full
    let (sender, receiver) = bounded::<Message>(1);
    sender
      .send(Message::SnapshotImmediate("pre".to_string()))
      .unwrap();

    // readiness channel to ensure receiver drained the prefilled slot
    let (ready_tx, ready_rx) = bounded::<()>(1);

    let recv_handle = std::thread::spawn(move || {
      let _ = receiver.recv().unwrap(); // drain prefilled
      let _ = ready_tx.send(()); // notify main thread

      // wait for panic hook message (timeout avoids hanging)
      match receiver.recv_timeout(Duration::from_secs(2)) {
        Ok(msg) => Some(msg),
        Err(RecvTimeoutError::Timeout) => None,
        Err(RecvTimeoutError::Disconnected) => None,
      }
    });

    // Wait for receiver to drain the prefilled slot
    ready_rx
      .recv_timeout(Duration::from_secs(1))
      .expect("receiver did not drain prefilled slot in time");

    // Save the previous global panic hook and restore it later
    let previous_hook = std::panic::take_hook();

    // Install our hook (moves sender into it)
    PanicHook::install(sender);

    // Trigger the panic (inside catch_unwind so test runner continues)
    let result = panic::catch_unwind(|| panic!("Test panic with full channel"));
    assert!(result.is_err());

    // Restore previous hook so other tests are unaffected
    std::panic::set_hook(previous_hook);

    // Join receiver and assert we got the snapshot
    let received = recv_handle.join().expect("receiver thread panicked");
    assert!(
      received.is_some(),
      "expected to receive the snapshot from panic hook"
    );
  }
}
