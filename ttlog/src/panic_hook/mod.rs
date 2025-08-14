use std::thread;

use crate::trace::Message;
use chrono::Duration;
use crossbeam_channel::Sender;

/// `
/// PanicHook` is a utility for installing a custom panic hook in Rust programs.
///
/// When a panic occurs, this hook captures the panic information and requests
/// an immediate snapshot to be sent via a `crossbeam_channel::Sender<Message>`.
/// This is useful in tracing or logging systems to capture the state of the
/// application at the moment of a panic.
///
/// # Example
///
/// ```rust
/// use crossbeam_channel::unbounded;
/// use crate::trace::Message;
/// use crate::PanicHook;
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
/// The hook will attempt to send a `Message::SnapshotImmediate` containing
/// the reason `"panic"` without blocking.
pub struct PanicHook {}

impl PanicHook {
  /// Installs a panic hook that requests an immediate snapshot when a panic occurs.
  ///
  /// # Parameters
  ///
  /// - `sender`: A `crossbeam_channel::Sender<Message>` used to request the snapshot.
  ///   The hook will use `try_send` to avoid blocking the panic unwinding process.
  ///
  /// # Behavior
  ///
  /// When a panic occurs:
  /// 1. The panic information (`std::panic::PanicInfo`) is printed to stderr.
  /// 2. A `Message::SnapshotImmediate("panic")` is sent through the provided sender.
  ///    If sending fails (e.g., channel is full or closed), the error is ignored.
  pub fn install(sender: Sender<Message>) {
    std::panic::set_hook(Box::new(move |info| {
      eprintln!("[Panic] Captured panic: {:?}", info);

      // Send snapshot request
      if let Err(e) = sender.try_send(Message::SnapshotImmediate("panic".to_string())) {
        eprintln!("[Panic] Failed to send snapshot request: {:?}", e);
        return;
      }

      eprintln!("[Panic] Snapshot request sent, waiting for completion...");

      // Give the writer thread time to process the snapshot
      // This is a blocking operation, but we're in a panic handler
      thread::sleep(Duration::milliseconds(100).to_std().unwrap());

      eprintln!("[Panic] Panic hook completed");
    }));
  }
}
