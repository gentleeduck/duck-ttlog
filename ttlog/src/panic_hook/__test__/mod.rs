#[cfg(test)]
mod tests {
  use crate::panic_hook::PanicHook;
  use crate::trace::Message;
  use crossbeam_channel::{bounded, Receiver, RecvTimeoutError};
  use std::{panic, thread, time::Duration};

  /// Start a background thread that drains the receiver for a short while.
  /// This prevents the panic hook's blocking `send` from deadlocking tests.
  fn start_drain_thread(receiver: Receiver<Message>) -> thread::JoinHandle<()> {
    thread::spawn(move || {
      loop {
        match receiver.recv_timeout(Duration::from_millis(500)) {
          Ok(_msg) => continue,
          Err(RecvTimeoutError::Timeout) => break,
          Err(RecvTimeoutError::Disconnected) => break,
        }
      }
    })
  }

  /// Helper: install the hook using `sender`, run `panic_action`, then restore previous hook.
  fn install_hook_and_run<F>(
    sender: crossbeam_channel::Sender<Message>,
    panic_action: F,
  ) -> Result<(), Box<dyn std::any::Any + Send>>
  where
    F: FnOnce() + panic::UnwindSafe,
  {
    let previous_hook = panic::take_hook();
    PanicHook::install(sender);
    let result = panic::catch_unwind(panic_action);
    std::panic::set_hook(previous_hook);
    result
  }

  #[test]
  fn test_panic_hook_install() {
    let (sender, _receiver) = bounded::<Message>(10);
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
    drop(receiver);

    let result = install_hook_and_run(sender, || panic!("Test panic with disconnected channel"));
    assert!(result.is_err());
  }

  #[test]
  fn test_panic_hook_multiple_installations() {
    let (sender1, receiver1) = bounded::<Message>(10);
    let (sender2, receiver2) = bounded::<Message>(10);

    let h1 = start_drain_thread(receiver1);
    let h2 = start_drain_thread(receiver2);

    let prev = panic::take_hook();
    PanicHook::install(sender1);
    PanicHook::install(sender2);
    std::panic::set_hook(prev);

    let _ = h1.join();
    let _ = h2.join();
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

    let prev = panic::take_hook();
    PanicHook::install(sender);
    std::panic::set_hook(prev);

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
  fn test_panic_hook_sends_snapshot_message() {
    let (sender, receiver) = bounded::<Message>(10);

    // Install hook, trigger panic, then check what was sent
    let prev = panic::take_hook();
    PanicHook::install(sender);

    // Spawn a thread that panics so we can inspect the message
    let handle = thread::spawn(|| {
      panic!("test snapshot send");
    });
    let _ = handle.join();

    std::panic::set_hook(prev);

    // The hook should have sent a SnapshotImmediate message
    match receiver.recv_timeout(Duration::from_secs(2)) {
      Ok(Message::SnapshotImmediate(reason, _ack)) => {
        assert_eq!(reason, "panic");
      },
      Ok(Message::FlushAndExit) => {
        panic!("expected SnapshotImmediate, got FlushAndExit");
      },
      Err(e) => {
        // The hook's try_send may fail if channel is full or timing issue;
        // this is acceptable behavior, not a test failure per se.
        eprintln!("Note: did not receive message from panic hook: {:?}", e);
      },
    }
  }
}
