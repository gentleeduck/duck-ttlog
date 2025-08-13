use tracing::info;
use ttlog::trace::Trace;

fn main() {
  let trace = Trace::init(5); // buffer size

  info!("First log message");
  info!("Second log message");
  info!("Third log message");

  println!("--- Buffer Contents ---");
  trace.print_logs();
}
