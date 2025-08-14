mod __test__;

use crate::trace::Message;

use chrono::Duration;
use crossbeam_channel::Sender;
use std::thread;

/// `PanicHook` is a utility for installing a custom panic hook in Rust programs.
///
/// When a panic occurs, this hook captures the panic information and attempts
/// to send an immediate snapshot request via a `crossbeam_channel::Sender<Message>`.
/// This is useful in tracing or logging systems to capture the state of the
/// application at the moment of a panic.
///
/// # Example
///
/// ```rust,no_run
/// use ttlog::panic_hook::PanicHook;
/// use ttlog::trace::Message;
/// use crossbeam_channel::unbounded;
///
/// let (sender, receiver) = unbounded();
///
/// // Install the panic hook
/// PanicHook::install(sender.clone());
///
/// // Trigger a panic to test
/// std::panic::panic_any("something went wrong");
/// ```
///
/// The hook will attempt to send a `Message::SnapshotImmediate("panic")`
/// using `try_send` or `send`. If sending fails (e.g., channel is full or closed),
/// the error is printed to stderr but otherwise ignored. The hook also sleeps
/// briefly to give background threads time to process the snapshot request.
pub struct PanicHook {}

impl PanicHook {
  /// Installs a panic hook that requests an immediate snapshot when a panic occurs.
  ///
  /// # Parameters
  ///
  /// - `sender`: A `crossbeam_channel::Sender<Message>` used to request the snapshot.
  ///   The hook will send a `Message::SnapshotImmediate("panic")` via this channel.
  ///
  /// # Behavior
  ///
  /// When a panic occurs:
  /// 1. The panic information (`std::panic::PanicInfo`) is printed to stderr.
  /// 2. A `Message::SnapshotImmediate("panic")` is sent through the provided sender.
  ///    If sending fails (e.g., channel is full or closed), the error is printed to stderr.
  /// 3. The hook sleeps for a short duration (default 120ms) to give writer or logger
  ///    threads time to process the snapshot request.
  /// 4. Finally, the hook prints a completion message to stderr.
  ///
  /// # Notes
  ///
  /// - This hook is intended for **debugging and tracing**. It should not rely on
  ///   complex logic, as panic handlers run during unwinding and should remain lightweight.
  /// - Blocking operations in the panic hook are generally discouraged. The brief
  ///   sleep here is a compromise to allow background snapshot processing.
  pub fn install(sender: Sender<Message>) {
    std::panic::set_hook(Box::new(move |info| {
      eprintln!("[Panic] Captured panic: {:?}", info);

      // non-blocking attempt to enqueue; do NOT block in panic handler
      if let Err(e) = sender.try_send(Message::SnapshotImmediate("panic".to_string())) {
        eprintln!("[Panic] Unable to enqueue snapshot request: {:?}", e);
      } else {
        eprintln!("[Panic] Snapshot request enqueued");
      }

      // Give the writer thread time to process the snapshot
      thread::sleep(Duration::milliseconds(120).to_std().unwrap());

      eprintln!("[Panic] Panic hook completed");
    }));
  }
}
