use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

use crate::buffer::RingBuffer;
use crate::event::Event;

// wrap metadata + events
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
  pub service: String,
  pub hostname: String,
  pub pid: u32,
  pub created_at: String,
  pub reason: String,
  pub events: Vec<Event>,
}
/// Responsible for creating snapshots from a ring buffer and writing them to disk.
pub struct SnapshotWriter {
  service: String,
}

impl SnapshotWriter {
  /// Creates a new `SnapshotWriter` for a given service name.
  pub fn new(service: impl Into<String>) -> Self {
    Self {
      service: service.into(),
    }
  }

  /// Take a snapshot from the ring buffer, capturing metadata and events.
  ///
  /// # Parameters
  /// - `ring`: The ring buffer containing events.
  /// - `reason`: Reason for taking the snapshot (for logging/audit).
  ///
  /// # Returns
  /// A `Snapshot` struct containing all events and metadata.
  pub fn create_snapshot(
    &self,
    ring: &mut RingBuffer<Event>,
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

  /// Serialize a snapshot to CBOR, compress with LZ4, and write it atomically to disk.
  ///
  /// # Parameters
  /// - `snapshot`: The snapshot to serialize and write.
  ///
  /// # Returns
  /// `Ok(())` if successful, otherwise an error.
  pub fn write_snapshot(&self, snapshot: &Snapshot) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize CBOR
    let cbor_buff = serde_cbor::to_vec(&snapshot)?;
    // Compress
    let compressed = compress(&cbor_buff, Some(CompressionMode::DEFAULT), true)?;

    // Build filename and write atomically
    let filename = format!(
      "/tmp/ttlog-{}-{}-{}.bin",
      snapshot.pid, snapshot.created_at, snapshot.reason
    );
    let tmp = format!("{}.tmp", &filename);

    {
      let mut f = File::create(&tmp)?;
      f.write_all(&compressed)?;
      f.sync_all()?;
    }
    fs::rename(&tmp, &filename)?;
    eprintln!(
      "[Snapshot] Saved {} events to {}",
      snapshot.events.len(),
      filename
    );
    Ok(())
  }

  /// Take a snapshot from the ring buffer, capturing metadata and events.
  /// If a snapshot is created, write it to disk.
  ///
  /// # Parameters
  /// - `ring`: The ring buffer containing events.
  /// - `reason`: Reason for taking the snapshot (for logging/audit).
  ///
  /// # Returns
  /// `Ok(())` if successful, otherwise an error.
  pub fn snapshot_and_write(
    &self,
    ring: &mut RingBuffer<Event>,
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
