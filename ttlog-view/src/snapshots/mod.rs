mod __test__;

use lz4::block::decompress;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs};
use ttlog::snapshot::SnapShot;

/// Hard upper bound on a decompressed snapshot payload. Anything larger is
/// treated as a decompression bomb and rejected before allocation.
pub const MAX_DECOMPRESSED_SIZE: usize = 256 * 1024 * 1024; // 256 MB

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
  pub name: String,
  pub path: String,
  pub create_at: String,
  pub data: SnapShot,
}

/// Reads the little-endian u32 size prefix that `lz4::block::compress(.., true)`
/// writes and refuses payloads claiming to expand beyond `MAX_DECOMPRESSED_SIZE`.
pub(crate) fn bounded_lz4_decompress(buf: &[u8]) -> Result<Vec<u8>, Box<dyn Error>> {
  if buf.len() < 4 {
    return Err("lz4 payload missing size prefix".into());
  }
  let claimed = u32::from_le_bytes([buf[0], buf[1], buf[2], buf[3]]) as usize;
  if claimed > MAX_DECOMPRESSED_SIZE {
    return Err(format!(
      "lz4 decompressed size {} exceeds limit {}",
      claimed, MAX_DECOMPRESSED_SIZE
    )
    .into());
  }
  // Pass `None` so the library reads the prepended size itself; we have already
  // bounded the claim above.
  Ok(decompress(buf, None)?)
}

pub struct Snapshots;

impl Snapshots {
  pub fn read_snapshots(path: &str) -> Result<Vec<SnapshotFile>, Box<dyn Error>> {
    // Get the snapshots
    let snapshots_dirs = fs::read_dir(path)?
      .filter_map(|e| e.ok())
      .filter(|e| e.file_name().to_string_lossy().starts_with("ttlog-"))
      .collect::<Vec<_>>();

    let mut snapshots: Vec<SnapshotFile> = vec![];
    for dir in &snapshots_dirs {
      // Read the snapshot dir
      let snapshot_compressed = fs::read(dir.path())?;
      // Decompress the snapshot with a hard upper bound to defeat
      // decompression bombs.
      let snapshot_decompressed = bounded_lz4_decompress(&snapshot_compressed)?;
      // Deserialize the snapshot
      let snapshot: SnapShot = serde_cbor::from_slice(&snapshot_decompressed)?;
      // Build data
      let path = dir.path().to_string_lossy().to_string();
      let path = path.strip_prefix("./tmp/").unwrap();
      let chunks = path.split('-').collect::<Vec<_>>();

      let (_prefix, _pid, tsz, _suffix) = match chunks.as_slice() {
        [prefix, pid, tsz, rest @ ..] => (prefix, pid, tsz, rest),
        _ => panic!("Expected at least 3 parts in filename"),
      };

      snapshots.push(SnapshotFile {
        name: path.strip_suffix(".bin").unwrap().to_string(),
        path: dir.path().to_string_lossy().to_string(),
        create_at: tsz.to_string(),
        data: snapshot,
      });
    }

    Ok(snapshots)
  }
}
