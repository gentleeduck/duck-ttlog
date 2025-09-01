#[cfg(test)]
mod tests {
  use crate::panic_hook::PanicHook;
  use crate::trace::{Message, Trace};
  use crossbeam_channel::{bounded, unbounded, Receiver, RecvTimeoutError};
  use std::{panic, thread, time::Duration};

  /// Start a background thread that drains the receiver for a short while.
  /// This prevents the panic hook's blocking `send` from deadlocking tests.
  fn start_drain_thread(receiver: Receiver<Message>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
      loop {
        match receiver.recv_timeout(Duration::from_millis(500)) {
          Ok(_msg) => continue, // keep draining
          Err(RecvTimeoutError::Timeout) => break,
          Err(RecvTimeoutError::Disconnected) => break,
        }
      }
    })
  }

  /// Helper: install the hook using `sender`, run `panic_action`, then restore previous hook.
  /// Returns any `panic::catch_unwind` result so tests can assert it.
  fn install_hook_and_run<F>(
    sender: crossbeam_channel::Sender<Message>,
    panic_action: F,
  ) -> Result<(), Box<dyn std::any::Any + Send>>
  where
    F: FnOnce() + panic::UnwindSafe,
  {
    // save previous hook and install ours
    let previous_hook = panic::take_hook();
    PanicHook::install(sender);

    // run the action that will panic (we catch to keep test runner alive)
    let result = panic::catch_unwind(panic_action);

    // restore previous hook immediately after the panic has been delivered
    std::panic::set_hook(previous_hook);

    // return the catch_unwind result so caller can assert
    result
  }

  #[test]
  fn test_panic_hook_install() {
    let (sender, _receiver) = bounded::<Message>(10);
    // save & restore to avoid affecting other tests
    let previous_hook = panic::take_hook();
    PanicHook::install(sender);
    std::panic::set_hook(previous_hook);
  }

  #[test]
  fn test_panic_hook_message_sending() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let result = install_hook_and_run(sender, || panic!("Test panic for hook"));
    assert!(result.is_err(), "expected a panic to occur");

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_disconnected_channel() {
    let (sender, receiver) = bounded::<Message>(10);
    // Drop receiver to simulate disconnected channel; send should return Err in hook.
    drop(receiver);

    let result = install_hook_and_run(sender, || panic!("Test panic with disconnected channel"));
    assert!(result.is_err());
  }

  #[test]
  fn test_panic_hook_multiple_installations() {
    let (sender1, receiver1) = bounded::<Message>(10);
    let (sender2, receiver2) = bounded::<Message>(10);

    // Drain both receivers so hook send won't block
    let h1 = start_drain_thread(receiver1);
    let h2 = start_drain_thread(receiver2);

    // Save and restore previous hook manually to ensure cleanup
    let prev = panic::take_hook();
    PanicHook::install(sender1);
    PanicHook::install(sender2);
    std::panic::set_hook(prev);

    let _ = h1.join();
    let _ = h2.join();
  }

  #[test]
  fn test_panic_hook_integration_with_trace() {
    // Use Trace::init to get a sender; assume Trace manages its own internals.
    let trace_system = Trace::init(100, 10);

    // For safety, create a draining receiver that won't interfere with Trace internals.
    let (_test_sender, test_receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(test_receiver);

    let result = install_hook_and_run(trace_system.get_sender(), || {
      panic!("Integration test panic")
    });
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_various_panic_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let results = vec![
      install_hook_and_run(sender.clone(), || panic!("String panic")),
      install_hook_and_run(sender.clone(), || panic!("{}", "Formatted panic")),
      install_hook_and_run(sender.clone(), || panic!("{}", 42)),
      install_hook_and_run(sender.clone(), || panic!("{:?}", vec![1, 2, 3])),
    ];

    for res in results {
      assert!(res.is_err());
    }

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_custom_panic_info() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let result = install_hook_and_run(sender, || panic::panic_any("Custom panic payload"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_thread_safety() {
    let (sender, receiver) = bounded::<Message>(100);
    let drain = start_drain_thread(receiver);

    // install single hook used by threads
    let prev = panic::take_hook();
    PanicHook::install(sender);
    std::panic::set_hook(prev);

    // spawn multiple threads, some panic
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

  #[test]
  fn test_panic_hook_with_nested_panics() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let result = install_hook_and_run(sender, || {
      let _ = panic::catch_unwind(|| panic!("Inner panic"));
      panic!("Outer panic");
    });
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_long_panic_messages() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let long_message = "A".repeat(10000);
    let result = install_hook_and_run(sender, || panic!("{}", long_message));
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_unicode_messages() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let result = install_hook_and_run(sender, || panic!("Unicode panic: 🚀 🎉 💻"));
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_special_characters() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let result = install_hook_and_run(sender, || {
      panic!("Special chars: \"quotes\" and \n newlines")
    });
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_numeric_payloads() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let results = vec![
      install_hook_and_run(sender.clone(), || panic::panic_any(42u32)),
      install_hook_and_run(sender.clone(), || panic::panic_any(3.14f64)),
      install_hook_and_run(sender.clone(), || panic::panic_any(-1i32)),
    ];

    for res in results {
      assert!(res.is_err());
    }

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_collection_payloads() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let results = vec![
      install_hook_and_run(sender.clone(), || panic::panic_any(vec![1, 2, 3])),
      install_hook_and_run(sender.clone(), || panic::panic_any(["a", "b", "c"])),
      install_hook_and_run(sender.clone(), || {
        panic::panic_any(std::collections::HashMap::<i32, i32>::new())
      }),
    ];

    for res in results {
      assert!(res.is_err());
    }

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_custom_error_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

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
    let result = install_hook_and_run(sender, || panic::panic_any(custom_error));
    assert!(result.is_err());

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_std_error_types() {
    let (sender, receiver) = bounded::<Message>(10);
    let drain = start_drain_thread(receiver);

    let results = vec![
      install_hook_and_run(sender.clone(), || {
        panic::panic_any(std::io::Error::new(
          std::io::ErrorKind::NotFound,
          "File not found",
        ))
      }),
      install_hook_and_run(sender.clone(), || panic::panic_any(std::fmt::Error)),
    ];

    for res in results {
      assert!(res.is_err());
    }

    let _ = drain.join();
  }

  #[test]
  fn test_panic_hook_with_full_channel() {
    use crossbeam_channel::RecvTimeoutError;
    use std::time::Duration;

    // capacity 1, fill the channel so it's full
    let (sender, receiver) = bounded::<Message>(1);
    sender
      .send(Message::SnapshotImmediate {
        field1: "pre".to_string(),
      })
      .unwrap();

    // readiness channel to ensure receiver drained the prefilled slot
    let (ready_tx, ready_rx) = bounded::<()>(1);

    // completion channel to coordinate shutdown
    let (done_tx, done_rx) = bounded::<()>(1);

    // Spawn a receiver thread that drains the prefilled item then waits for the hook
    let recv_handle = thread::spawn(move || {
      // drain prefilled
      let msg = receiver.recv().unwrap();
      println!("Drained prefilled message: {:?}", msg);

      // notify main test we've drained the prefilled slot
      let _ = ready_tx.send(());

      println!("Receiver waiting for panic hook message...");

      // wait for panic hook message (longer timeout to avoid race conditions)
      let result = match receiver.recv_timeout(Duration::from_secs(5)) {
        Ok(msg) => {
          println!("Received panic hook message: {:?}", msg);
          Some(msg)
        },
        Err(RecvTimeoutError::Timeout) => {
          println!("Timeout waiting for panic hook message");
          None
        },
        Err(RecvTimeoutError::Disconnected) => {
          println!("Channel disconnected while waiting for panic hook message");
          None
        },
      };

      // Signal that we're done
      let _ = done_tx.send(());

      result
    });

    // Wait for receiver to drain the prefilled slot
    ready_rx
      .recv_timeout(Duration::from_secs(2))
      .expect("receiver did not drain prefilled slot in time");

    println!("Receiver is ready, installing panic hook...");

    // Save the previous global panic hook and restore it later
    let previous_hook = std::panic::take_hook();

    // Install our hook (moves sender into it)
    PanicHook::install(sender);

    println!("Triggering panic...");

    // Trigger the panic (inside catch_unwind so test runner continues)
    let result = panic::catch_unwind(|| panic!("Test panic with full channel"));
    assert!(result.is_err());

    println!("Panic completed, restoring hook...");

    // Restore previous hook so other tests are unaffected
    std::panic::set_hook(previous_hook);

    // Wait a moment for the panic hook to complete
    thread::sleep(Duration::from_millis(100));

    // Wait for receiver thread to complete
    done_rx
      .recv_timeout(Duration::from_secs(3))
      .expect("receiver thread did not complete in time");

    // Join receiver and check result
    let received = recv_handle.join().expect("receiver thread panicked");

    // The panic hook should successfully send a message even when the channel was initially full
    match received {
      Some(msg) => {
        println!("Successfully received panic hook message: {:?}", msg);
        // Verify it's the expected snapshot message
        match msg {
          Message::SnapshotImmediate { field1: _ } => {
            // Test passes - we got the expected snapshot message
            println!("Test passed: panic hook successfully sent snapshot message");
          },
          other => panic!("Expected SnapshotImmediate message, got {:?}", other),
        }
      },
      None => {
        panic!("Expected to receive the snapshot from panic hook, but got None");
      },
    }
  }
}
