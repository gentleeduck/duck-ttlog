mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;

use crate::event::LogEvent;
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
  pub service: String,
  pub hostname: String,
  pub pid: u32,
  pub created_at: String,
  pub reason: String,
  pub events: Vec<LogEvent>,
}

#[derive(Debug, Clone)]
pub struct SnapshotWriter {
  service: String,
  storage_path: String,
}

impl SnapshotWriter {
  pub fn new(service: impl Into<String>, storage_path: impl Into<String>) -> Self {
    Self {
      service: service.into(),
      storage_path: storage_path.into(),
    }
  }

  pub fn create_snapshot(
    &self,
    ring: &mut Arc<RingBuffer<LogEvent>>,
    reason: impl Into<String>,
  ) -> Option<Snapshot> {
    let events = ring.take_snapshot();
    if events.is_empty() {
      return None;
    }

    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
    let pid = std::process::id();
    let created_at = Utc::now().format("%Y%m%d%H%M%S").to_string();

    Some(Snapshot {
      service: self.service.clone(),
      hostname,
      pid,
      created_at,
      reason: reason.into(),
      events,
    })
  }

  pub fn write_snapshot(&self, snapshot: &Snapshot) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize CBOR
    let cbor_buff = serde_cbor::to_vec(&snapshot)?;
    // Compress
    let compressed = compress(&cbor_buff, Some(CompressionMode::DEFAULT), true)?;

    let path = if self.storage_path == "" {
      eprintln!("[Snapshot] No storage path set");
      "./tmp/".to_string()
    } else {
      self.storage_path.clone()
    };

    // Build filename and write atomically
    let filename = format!(
      "{}/ttlog-{}-{}-{}.bin",
      path, snapshot.pid, snapshot.created_at, snapshot.reason
    );

    // Ensure directory exists
    std::fs::create_dir_all(&path)?;

    {
      let mut f = File::create(&filename)?;
      f.write_all(&compressed)?;
      f.sync_all()?;
    }

    fs::rename(&filename, &filename)?;
    eprintln!(
      "[Snapshot] Saved {} events to {}",
      snapshot.events.len(),
      filename
    );
    Ok(())
  }

  pub fn snapshot_and_write(
    &self,
    ring: &mut Arc<RingBuffer<LogEvent>>,
    reason: impl Into<String>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(snapshot) = self.create_snapshot(ring, reason) {
      self.write_snapshot(&snapshot)
    } else {
      println!("[Snapshot] No events to snapshot");
      Ok(())
    }
  }
}
