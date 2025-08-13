mod __test__;
use std::{
  panic,
  sync::{Arc, Mutex},
};

use crate::{buffer::RingBuffer, event::Event, trace::Trace};

pub struct PanicHook {}

impl PanicHook {
  /// The install function sets up a custom panic handler for the current Application process
  pub fn install(buffer: Arc<Mutex<RingBuffer<Event>>>) {
    panic::set_hook(Box::new(move |info| {
      eprintln!("[Panic] Captured panic: {:?}", info);
      Trace::flush_snapshot(buffer.clone(), "panic");
    }));
  }
}
