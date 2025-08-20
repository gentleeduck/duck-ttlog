mod __test__;

use crate::trace::Message;

use chrono::Duration;
use crossbeam_channel::Sender;
use std::thread;

pub struct PanicHook {}

impl PanicHook {
  pub fn install(sender: Sender<Message>) {
    std::panic::set_hook(Box::new(move |info| {
      eprintln!("[Panic] Captured panic: {:?}", info);

      // non-blocking attempt to enqueue; do NOT block in panic handler
      if let Err(e) = sender.try_send(Message::SnapshotImmediate("panic")) {
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
