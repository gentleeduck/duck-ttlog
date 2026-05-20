mod __test__;
pub mod decompress;

pub use decompress::{bounded_lz4_decompress, MAX_DECOMPRESSED_SIZE};

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs;
use std::io::Write;
use std::path::Path;
use std::sync::Arc;

use crate::event::{LogEvent, LogLevel};
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;
use crate::string_interner::StringInterner;

/// Sanitises a snapshot `reason` for safe use as a filename component.
///
/// - Keeps only ASCII alphanumerics, `_`, and `-`.
/// - Drops every other character (notably `/`, `\`, `.`, NUL, control bytes).
/// - Truncates to 64 characters.
/// - Returns `"unknown"` when the result would otherwise be empty.
pub(crate) fn sanitize_reason(reason: &str) -> String {
  let cleaned: String = reason
    .chars()
    .filter(|c| c.is_ascii_alphanumeric() || *c == '_' || *c == '-')
    .take(64)
    .collect();
  if cleaned.is_empty() {
    "unknown".to_string()
  } else {
    cleaned
  }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct SnapShot {
  pub service: String,
  pub hostname: String,
  pub pid: u32,
  pub created_at: String,
  pub reason: String,
  pub events: Vec<ResolvedEvent>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ResolvedEvent {
  pub packed_meta: u64,
  pub message: String,
  pub target: String,
  pub kv: serde_json::Value,
  pub file: String,
  pub position: (u32, u32),
}

impl ResolvedEvent {
  pub fn timestamp_millis(&self) -> u64 {
    LogEvent::unpack_meta(self.packed_meta).0
  }

  pub fn level(&self) -> LogLevel {
    let (_, level, _) = LogEvent::unpack_meta(self.packed_meta);
    LogLevel::from_u8(&level)
  }
}

#[derive(Debug, Clone)]
pub struct SnapshotWriter {
  service: Cow<'static, str>,
  storage_path: Cow<'static, str>,
  /// When `false` (the default), the machine hostname is omitted from
  /// snapshots so shared CI / multi-tenant hosts do not leak metadata.
  include_hostname: bool,
}

impl SnapshotWriter {
  pub fn new(service: impl Into<String>) -> Self {
    Self::with_storage_path(service, "./tmp/")
  }

  pub fn with_storage_path(service: impl Into<String>, storage_path: impl Into<String>) -> Self {
    Self {
      service: Cow::Owned(service.into()),
      storage_path: Cow::Owned(storage_path.into()),
      include_hostname: false,
    }
  }

  /// Opts in to embedding the machine hostname in written snapshots.
  ///
  /// Off by default to avoid leaking host metadata on shared infrastructure.
  pub fn with_hostname(mut self, include_hostname: bool) -> Self {
    self.include_hostname = include_hostname;
    self
  }

  pub fn create_snapshot(
    &self,
    ring: &mut Arc<RingBuffer<LogEvent>>,
    reason: impl Into<String>,
    interner: Arc<StringInterner>,
  ) -> Option<SnapShot> {
    let events: Vec<ResolvedEvent> = ring
      .take_snapshot()
      .iter()
      .filter_map(|event| {
        // Try to get all required values, early return None if missing
        let message = match event
          .message_id
          .and_then(|id| interner.get_message(id.get()))
        {
          Some(m) => m.to_string(),
          None => {
            eprintln!("[Trace] Unknown message id: {:?}", event.message_id);
            return None;
          },
        };

        let target = match interner.get_target(event.target_id) {
          Some(t) => t.to_string(),
          None => {
            eprintln!("[Trace] Unknown target id: {}", event.target_id);
            return None;
          },
        };

        let kv = event.kv_id.and_then(|id| interner.get_kv(id.get()));
        let kv_data = if let Some(kv_bytes) = kv {
          if let Ok(kv_str) = std::str::from_utf8(&kv_bytes) {
            if let Ok(parsed) = serde_json::from_str::<serde_json::Value>(kv_str) {
              serde_json::json!(&parsed)
            } else {
              serde_json::json!({})
            }
          } else {
            serde_json::json!({})
          }
        } else {
          serde_json::json!({})
        };

        let file = match interner.get_file(event.file_id) {
          Some(f) => f.to_string(),
          None => {
            eprintln!("[Trace] Unknown file id: {}", event.file_id);
            return None;
          },
        };

        Some(ResolvedEvent {
          packed_meta: event.packed_meta,
          position: event.position,
          file,
          message,
          target,
          kv: kv_data,
        })
      })
      .collect();

    if events.is_empty() {
      return None;
    }

    // Hostname is opt-in: shared/multi-tenant hosts should not leak it.
    let hostname = if self.include_hostname {
      gethostname::gethostname().to_string_lossy().into_owned()
    } else {
      String::new()
    };
    let pid = std::process::id();
    let created_at = Utc::now().format("%Y%m%d%H%M%S").to_string();

    Some(SnapShot {
      service: self.service.to_string(),
      hostname,
      pid,
      created_at,
      reason: reason.into(),
      events,
    })
  }

  pub fn write_snapshot(&self, snapshot: &SnapShot) -> Result<(), Box<dyn std::error::Error>> {
    // Serialize CBOR
    let cbor_buff = serde_cbor::to_vec(&snapshot)?;
    // Compress
    let compressed = compress(&cbor_buff, Some(CompressionMode::DEFAULT), true)?;

    let path = if self.storage_path.is_empty() {
      eprintln!("[Snapshot] No storage path set");
      "./tmp/".to_string()
    } else {
      self.storage_path.to_string()
    };

    // Build filename and write atomically. The `reason` is sanitised to a
    // restricted alphabet to prevent path-traversal via `../`, NUL bytes, or
    // separators that could escape `storage_path`.
    let safe_reason = sanitize_reason(&snapshot.reason);
    let filename = Path::new(&path).join(format!(
      "ttlog-{}-{}-{}.bin",
      snapshot.pid, snapshot.created_at, safe_reason
    ));

    // Ensure directory exists
    std::fs::create_dir_all(&path)?;

    {
      // Snapshots may contain sensitive log data.
      //
      // On Unix the snapshot file is created with mode 0o600 (owner
      // read/write only). On Windows the file inherits the parent
      // directory's ACL — callers on Windows must restrict the storage
      // directory themselves.
      let mut opts = std::fs::OpenOptions::new();
      opts.write(true).create(true).truncate(true);
      #[cfg(unix)]
      {
        use std::os::unix::fs::OpenOptionsExt;
        opts.mode(0o600);
      }
      let mut f = opts.open(&filename)?;
      f.write_all(&compressed)?;
      f.sync_all()?;
    }

    fs::rename(&filename, &filename)?;
    eprintln!(
      "[Snapshot] Saved {} events to {}",
      snapshot.events.len(),
      filename.display()
    );
    Ok(())
  }

  pub fn snapshot_and_write(
    &self,
    ring: &mut Arc<RingBuffer<LogEvent>>,
    reason: impl Into<String>,
    interner: Arc<StringInterner>,
  ) -> Result<(), Box<dyn std::error::Error>> {
    if let Some(snapshot) = self.create_snapshot(ring, reason, interner) {
      self.write_snapshot(&snapshot)
    } else {
      println!("[Snapshot] No events to snapshot");
      Ok(())
    }
  }
}
