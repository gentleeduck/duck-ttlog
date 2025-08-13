use tracing::{info, warn};
use ttlog::init;

fn main() {
  let buffer = init(10);

  info!("App started");
  warn!("Low disk space");
  info!("Shutting down");

  println!("--- Buffer contents ---");
  let buf = buffer.lock().unwrap();
  for ev in buf.iter() {
    println!("{:?}", ev);
  }
}
