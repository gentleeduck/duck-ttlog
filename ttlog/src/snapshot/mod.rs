mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

use crate::buffer::RingBuffer;
use crate::event::Event;

/// A snapshot bundles metadata together with a sequence of events.
///
/// A `Snapshot` is intended to be a compact, self-contained representation
/// of the recent runtime state of the service. It contains:
///
/// * `service` — logical name of the service creating the snapshot.
/// * `hostname` — platform host the snapshot was taken on.
/// * `pid` — process id of the running process that created the snapshot.
/// * `created_at` — timestamp string (formatted `YYYYMMDDHHMMSS`) when snapshot was created.
/// * `reason` — human-readable reason for the snapshot (e.g., `"panic"`, `"manual"`).
/// * `events` — the captured events from the ring buffer (oldest → newest).
#[derive(Serialize, Deserialize, Clone, Debug)]
pub struct Snapshot {
  /// Name of the service (e.g., `"ttlog"`).
  pub service: String,
  /// Hostname where the snapshot was captured.
  pub hostname: String,
  /// OS process id that created the snapshot.
  pub pid: u32,
  /// Snapshot creation timestamp formatted as `YYYYMMDDHHMMSS`.
  pub created_at: String,
  /// Reason for taking the snapshot (free-form string).
  pub reason: String,
  /// Captured events (in insertion order).
  pub events: Vec<Event>,
}

/// Writes `Snapshot` instances to disk.
///
/// `SnapshotWriter` is a small helper that:
/// 1. extracts events from a `RingBuffer<Event>`,
/// 2. marshals the `Snapshot` into CBOR,
/// 3. compresses the CBOR payload with LZ4 (block API),
/// 4. writes the compressed bytes atomically to disk (write to `.tmp` then rename).
///
/// The on-disk format is intentionally simple: CBOR payload compressed with LZ4.
/// The file naming scheme is:
///
/// ```text
/// /tmp/ttlog-<pid>-<created_at>-<reason>.bin
/// ```
///
/// where `<created_at>` uses `YYYYMMDDHHMMSS`.
pub struct SnapshotWriter {
  service: String,
}

impl SnapshotWriter {
  /// Create a new `SnapshotWriter`.
  ///
  /// # Arguments
  ///
  /// * `service` - logical name of the service that will appear in every snapshot.
  ///
  /// # Example (illustrative)
  ///
  /// ```rust,ignore
  /// // create a writer bound to an application name
  /// let writer = SnapshotWriter::new("my-service");
  /// ```
  pub fn new(service: impl Into<String>) -> Self {
    Self {
      service: service.into(),
    }
  }

  /// Create a snapshot from the provided ring buffer.
  ///
  /// This method **consumes** the current contents of the ring buffer by calling
  /// `take_snapshot()` on the ring. If the buffer is empty this returns `None`.
  ///
  /// The snapshot contains:
  /// * the provided `reason` (converted to `String`),
  /// * hostname via `gethostname::gethostname()` (lossy string),
  /// * current `pid`,
  /// * `created_at` timestamp using UTC formatted `YYYYMMDDHHMMSS`,
  /// * all captured events.
  ///
  /// # Returns
  ///
  /// * `Some(Snapshot)` if there are events to snapshot.
  /// * `None` if the ring buffer was empty.
  ///
  /// # Notes
  ///
  /// * `take_snapshot()` swaps out the internal buffer to avoid reallocations.
  /// * This function is synchronous and cheap — it mainly constructs metadata
  ///   and moves the event vector out of the `RingBuffer`.
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
