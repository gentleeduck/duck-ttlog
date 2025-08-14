use lz4::block::decompress;
use serde::{Deserialize, Serialize};
use std::{error::Error, fs};
use ttlog::event::Event;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SnapShot {
  pub name: String,
  pub path: String,
  pub create_at: String,
  pub data: Vec<Event>,
}

pub fn read_snapshots() -> Result<Vec<SnapShot>, Box<dyn Error>> {
  // Get the snapshots
  let snapshots_dirs = fs::read_dir("/tmp")?
    .filter_map(|e| e.ok())
    .filter(|e| e.file_name().to_string_lossy().starts_with("ttlog-"))
    .collect::<Vec<_>>();

  let mut snapshots: Vec<SnapShot> = vec![];
  for dir in &snapshots_dirs {
    // Read the snapshot dir
    let snapshot_compressed = fs::read(&dir.path())?;
    // Decompress the snapshot
    let snapshot_decompressed = decompress(&snapshot_compressed, None)?;
    // Deserialize the snapshot
    let snapshot: Vec<Event> = serde_cbor::from_slice(&snapshot_decompressed)?;
    // Build data
    let path = dir.path().to_string_lossy().to_string();
    let path = path.strip_prefix("/tmp/").unwrap();
    let chunks = path.split("-").into_iter().collect::<Vec<_>>();
    let (_prefix, _pid, tsz, _suffix) = match chunks.as_slice() {
      [prefix, pid, tsz, suffix] => (prefix, pid, tsz, suffix),
      _ => panic!("Expected 4 parts in filename"),
    };

    snapshots.push(SnapShot {
      name: path.strip_suffix(".bin").unwrap().to_string(),
      path: dir.path().to_string_lossy().to_string(),
      create_at: tsz.to_string(),
      data: snapshot,
    });
  }

  Ok(snapshots)
}
