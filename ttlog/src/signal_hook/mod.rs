// mod __test__;

use chrono::Duration;
use crossbeam_channel::Sender;
use signal_hook::{
  consts::{
    SIGABRT, SIGBUS, SIGCHLD, SIGFPE, SIGHUP, SIGILL, SIGINT, SIGPIPE, SIGQUIT, SIGSEGV, SIGTERM,
  },
  iterator::Signals,
};
use std::thread;

use crate::trace::Message;

pub struct SignalHook {}

impl SignalHook {
  pub fn install(sender: Sender<Message>) {
    let mut signals = match Signals::new(&[
      SIGINT, SIGTERM, SIGQUIT, SIGHUP, SIGABRT, SIGSEGV, SIGBUS, SIGILL, SIGFPE, SIGPIPE, SIGCHLD,
    ]) {
      Ok(s) => s,
      Err(e) => panic!("Failed to install signal handler: {}", e),
    };

    thread::spawn(move || {
      for sig in signals.forever() {
        match sig {
          SIGINT => SignalHook::signal_request_snapshot(&sender, "SIGINT"),
          SIGTERM => SignalHook::signal_request_snapshot(&sender, "SIGTERM"),
          SIGQUIT => SignalHook::signal_request_snapshot(&sender, "SIGQUIT"),
          SIGHUP => SignalHook::signal_request_snapshot(&sender, "SIGHUP"),
          SIGABRT => SignalHook::signal_request_snapshot(&sender, "SIGABRT"),
          SIGSEGV => SignalHook::signal_request_snapshot(&sender, "SIGSEGV"),
          SIGBUS => SignalHook::signal_request_snapshot(&sender, "SIGBUS"),
          SIGILL => SignalHook::signal_request_snapshot(&sender, "SIGILL"),
          SIGFPE => SignalHook::signal_request_snapshot(&sender, "SIGFPE"),
          SIGPIPE => SignalHook::signal_request_snapshot(&sender, "SIGPIPE"),
          SIGCHLD => SignalHook::signal_request_snapshot(&sender, "SIGCHLD"),
          _ => unreachable!(),
        }
        // do cleanup or exit here if needed
      }
    });
  }

  fn signal_request_snapshot(sender: &Sender<Message>, info: &str) {
    eprintln!("[{}] Captured panic: {:?}", info, info);

    // non-blocking attempt to enqueue; do NOT block in panic handler
    if let Err(e) = sender.try_send(Message::SnapshotImmediate("panic")) {
      eprintln!("[{}] Unable to enqueue snapshot request: {:?}", e, info);
    } else {
      eprintln!("[{}] Snapshot request enqueued", info);
    }

    // Give the writer thread time to process the snapshot
    thread::sleep(Duration::milliseconds(120).to_std().unwrap());

    eprintln!("[{}] Panic hook completed", info);
  }
}
