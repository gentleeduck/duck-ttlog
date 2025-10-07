use lz4::block::decompress;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs};
use ttlog::snapshot::SnapShot;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapshotFile {
  pub name: String,
  pub path: String,
  pub create_at: String,
  pub data: SnapShot,
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
      let snapshot_compressed = fs::read(&dir.path())?;
      // Decompress the snapshot
      let snapshot_decompressed = decompress(&snapshot_compressed, None)?;
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
