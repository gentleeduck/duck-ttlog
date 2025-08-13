use tracing::info;
use ttlog::panic_hook::PanicHook;
use ttlog::trace::Trace;

fn main() {
  let trace = Trace::init(5);
  let buffer = trace.get_buffer();

  PanicHook::install(buffer.clone());

  info!("First log message");
  info!("Second log message");
  info!("Third log message");

  // Trace::flush_snapshot(buffer.clone(), "main");

  // Trigger panic
  panic!("This is a test panic!");
}
