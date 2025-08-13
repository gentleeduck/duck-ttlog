mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use std::fs::File;
use std::io::Write;
use std::sync::{Arc, Mutex};
use tracing_subscriber::layer::SubscriberExt;
use tracing_subscriber::Registry;

use crate::buffer::RingBuffer;
use crate::event::Event;
use crate::trace_layer::BufferLayer;

pub struct Trace {
  pub buffer: Arc<Mutex<RingBuffer<Event>>>,
}

impl Trace {
  pub fn init(capacity: usize) -> Self {
    let buffer = Arc::new(Mutex::new(RingBuffer::new(capacity)));
    let layer = BufferLayer::new(buffer.clone());

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::set_global_default(subscriber)
      .expect("Failed to set global tracing subscriber");

    Self { buffer }
  }

  pub fn get_buffer(&self) -> Arc<Mutex<RingBuffer<Event>>> {
    self.buffer.clone()
  }

  pub fn flush_snapshot(buffer: Arc<Mutex<RingBuffer<Event>>>, reason: &str) {
    // Check for the buffer
    let buf = buffer.lock().unwrap().iter().cloned().collect::<Vec<_>>();
    if buf.is_empty() {
      return;
    }

    // Serialize the Buffer to Concise Binary Object Representation ( CBOR )
    let cbor_buff = match serde_cbor::to_vec(&buf) {
      Ok(buff) => buff,
      Err(e) => {
        println!("Failed to serialize snapshot: {}", e);
        return;
      },
    };

    // NOTE: We can check for more high performance compression
    let compressed_buff = match compress(&cbor_buff, Some(CompressionMode::DEFAULT), true) {
      Ok(buff) => buff,
      Err(e) => {
        println!("Failed to compress snapshot: {}", e);
        return;
      },
    };

    // Build the file Path
    let pid = std::process::id();
    let timestamps = Utc::now().format("%Y%m%d%H%M%S");
    let filename = format!("/tmp/ttlog-{}-{}-{}.bin", pid, timestamps, reason);

    // Write the file
    if let Err(e) = File::create(&filename).and_then(|mut f| f.write_all(&compressed_buff)) {
      eprintln!("[Snapshot] Failed to write file {}: {}", filename, e);
    } else {
      eprintln!("[Snapshot] Saved {} events to {}", buf.len(), filename);
    }
  }
}
