mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

use crate::event::LogEvent;
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;

/// A snapshot bundles metadata together with a sequence of events.
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
}

impl SnapshotWriter {
  pub fn new(service: impl Into<String>) -> Self {
    Self {
      service: service.into(),
    }
  }

  pub fn create_snapshot(
    &self,
    ring: &mut RingBuffer<LogEvent>,
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

  /// Serialize a `Snapshot` to CBOR, compress it with LZ4, and write it atomically.
  ///
  /// Steps:
  /// 1. Serialize the `Snapshot` to CBOR using `serde_cbor::to_vec`.
  /// 2. Compress the CBOR bytes with `lz4::block::compress`.
  /// 3. Write to a temporary file `<filename>.tmp` and call `sync_all()` to flush.
  /// 4. Rename the temp file to the final filename (atomic on POSIX within the same FS).
  ///
  /// # Returns
  ///
  /// * `Ok(())` on success.
  /// * `Err(Box<dyn std::error::Error>)` if any IO/serialization/compression step fails.
  ///
  /// # File Naming
  ///
  /// Default path: `/tmp/ttlog-<pid>-<created_at>-<reason>.bin`
  ///
  /// You can change this behavior by modifying the filename construction in this method.
  ///
  /// # Errors & Safety
  ///
  /// * Serialization (`serde_cbor`) may fail for unexpected data — this returns an error.
  /// * Compression can fail — that error is propagated.
  /// * File creation, `write_all`, `sync_all`, and `rename` can fail due to permissions, disk
  ///   errors, or lack of space. The method returns the boxed error in those cases.
  ///
  /// # Atomicity
  ///
  /// Writing to a temporary file and renaming it is the common POSIX pattern for atomic writes.
  /// On non-POSIX platforms the behavior of `fs::rename` may differ; adjust as necessary.
  ///
  /// # Example (illustrative; not compiled)
  ///
  /// ```rust,ignore
  /// let writer = SnapshotWriter::new("my-service");
  /// if let Some(snapshot) = writer.create_snapshot(&mut ring, "manual") {
  ///     writer.write_snapshot(&snapshot)?;
  /// }
  /// ```
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

  /// Convenience: create a snapshot from `ring` and write it to disk if non-empty.
  ///
  /// This calls `create_snapshot()` and — when it returns `Some` — forwards the snapshot
  /// to `write_snapshot()`. If there are no events the method returns `Ok(())` and
  /// prints a small message.
  ///
  /// # Example (illustrative; not compiled)
  ///
  /// ```rust,ignore
  /// let mut ring = RingBuffer::new(1024);
  /// // ... push events ...
  /// let writer = SnapshotWriter::new("svc");
  /// writer.snapshot_and_write(&mut ring, "periodic")?;
  /// ```
  pub fn snapshot_and_write(
    &self,
    ring: &mut RingBuffer<LogEvent>,
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
