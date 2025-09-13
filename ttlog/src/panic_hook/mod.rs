use crate::trace::Message;

use chrono::Duration;
use crossbeam_channel::Sender;
use std::thread;

pub struct PanicHook {}

impl PanicHook {
  pub fn install(sender: Sender<Message>) {
    std::panic::set_hook(Box::new(move |info| {
      eprintln!("[Panic] Captured panic: {:?}", info);

      // Create response channel
      let (tx, rx) = std::sync::mpsc::channel();

      // Try to enqueue snapshot request
      if let Err(e) = sender.try_send(Message::SnapshotImmediate("panic".to_string(), tx)) {
        eprintln!("[Panic] Unable to enqueue snapshot request: {:?}", e);
        return;
      }

      eprintln!("[Panic] Waiting for snapshot completion...");

      match rx.recv() {
        Ok(_) => eprintln!("[Panic] Snapshot completed!"),
        Err(err) => eprintln!("[Panic] Failed to receive snapshot confirmation: {:?}", err),
      }

      // Optional: small grace period to flush I/O
      thread::sleep(Duration::milliseconds(50).to_std().unwrap());

      eprintln!("[Panic] Panic hook completed");
    }));
  }
}
