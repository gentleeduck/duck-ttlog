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
    eprintln!("[{}] Captured signal: {:?}", info, info);

    // Create a response channel
    let (tx, rx) = std::sync::mpsc::channel();

    // Try to send snapshot request
    if let Err(e) = sender.try_send(Message::SnapshotImmediate(info.to_string(), tx)) {
      eprintln!("[{}] Failed to enqueue snapshot request: {:?}", info, e);
      return;
    }

    eprintln!("[{}] Waiting for snapshot completion...", info);

    // Wait for confirmation (blocks until snapshot thread responds)
    match rx.recv() {
      Ok(_) => eprintln!("[{}] Snapshot completed!", info),
      Err(err) => eprintln!(
        "[{}] Failed to receive snapshot confirmation: {:?}",
        info, err
      ),
    }

    eprintln!("[{}] Signal handling finished", info);
  }
}
