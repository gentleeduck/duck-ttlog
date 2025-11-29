mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::borrow::Cow;
use std::fs::{self, File};
use std::io::Write;
use std::sync::Arc;

use crate::event::LogEvent;
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;
use crate::string_interner::StringInterner;

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

#[derive(Debug, Clone)]
pub struct SnapshotWriter {
  service: Cow<'static, str>,
  storage_path: Cow<'static, str>,
}

impl SnapshotWriter {
  pub fn new(service: impl Into<String>, storage_path: impl Into<String>) -> Self {
    Self {
      service: Cow::Owned(service.into()),
      storage_path: Cow::Owned(storage_path.into()),
    }
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

    let hostname = gethostname::gethostname().to_string_lossy().into_owned();
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
