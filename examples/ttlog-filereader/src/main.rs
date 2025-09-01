// examples/snapshot_reader.rs
//
// Utility to read and display the contents of ttlog snapshot files

use lz4::block::decompress;
use serde::{Deserialize, Serialize};
use serde_cbor;
use std::env;
use std::fs::File;
use std::io::Read;

// Copy of the snapshot structure from your lib
#[derive(Serialize, Deserialize, Clone, Debug)]
struct Snapshot {
  pub service: String,
  pub hostname: String,
  pub pid: u32,
  pub created_at: String,
  pub reason: String,
  pub events: Vec<Event>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct Event {
  pub timestamp: u64,
  pub level: String,
  pub message: String,
  pub target: String,
}

fn read_snapshot_file(file_path: &str) -> Result<Snapshot, Box<dyn std::error::Error>> {
  println!("Reading snapshot file: {}", file_path);

  // Read the compressed file
  let mut file = File::open(file_path)?;
  let mut compressed_data = Vec::new();
  file.read_to_end(&mut compressed_data)?;

  println!("Compressed file size: {} bytes", compressed_data.len());

  // Decompress with LZ4
  let decompressed = decompress(&compressed_data, None)?;
  println!("Decompressed size: {} bytes", decompressed.len());

  // Deserialize from CBOR
  let snapshot: Snapshot = serde_cbor::from_slice(&decompressed)?;

  Ok(snapshot)
}

fn display_snapshot(snapshot: &Snapshot) {
  println!("\n=== SNAPSHOT DETAILS ===");
  println!("Service: {}", snapshot.service);
  println!("Hostname: {}", snapshot.hostname);
  println!("PID: {}", snapshot.pid);
  println!("Created At: {}", snapshot.created_at);
  println!("Reason: {}", snapshot.reason);
  println!("Event Count: {}", snapshot.events.len());

  println!("\n=== EVENTS ===");
  for (i, event) in snapshot.events.iter().enumerate() {
    println!(
      "Event #{}: [{}] {} - {} ({})",
      i + 1,
      event.level,
      format_timestamp(event.timestamp),
      event.message,
      event.target
    );
  }
}

fn format_timestamp(timestamp: u64) -> String {
  use std::time::{Duration, SystemTime, UNIX_EPOCH};

  let system_time = UNIX_EPOCH + Duration::from_millis(timestamp);
  format!("{:?}", system_time)
}

fn list_snapshot_files() -> std::io::Result<Vec<String>> {
  let mut snapshot_files = Vec::new();

  for entry in std::fs::read_dir("/tmp")? {
    let entry = entry?;
    let file_name = entry.file_name().to_string_lossy().to_string();

    if file_name.starts_with("ttlog-") && file_name.ends_with(".bin") {
      snapshot_files.push(format!("/tmp/{}", file_name));
    }
  }

  snapshot_files.sort();
  Ok(snapshot_files)
}

fn main() {
  let args: Vec<String> = env::args().collect();

  if args.len() < 2 {
    println!("TTLog Snapshot Reader");
    println!("Usage: {} <snapshot_file_path>", args[0]);
    println!("   or: {} --list", args[0]);
    println!();

    // List available files
    match list_snapshot_files() {
      Ok(files) => {
        if files.is_empty() {
          println!("No snapshot files found in /tmp/");
        } else {
          println!("Available snapshot files in /tmp/:");
          for file in files {
            println!("  {}", file);
          }
        }
      },
      Err(e) => {
        eprintln!("Error listing files: {}", e);
      },
    }
    return;
  }

  let arg = &args[1];

  if arg == "--list" {
    match list_snapshot_files() {
      Ok(files) => {
        if files.is_empty() {
          println!("No snapshot files found in /tmp/");
        } else {
          println!("Snapshot files found:");
          for file in files {
            println!("  {}", file);

            // Try to read basic info from each file
            match read_snapshot_file(&file) {
              Ok(snapshot) => {
                println!(
                  "    -> {} events, reason: {}, created: {}",
                  snapshot.events.len(),
                  snapshot.reason,
                  snapshot.created_at
                );
              },
              Err(e) => {
                println!("    -> Error reading: {}", e);
              },
            }
            println!();
          }
        }
      },
      Err(e) => {
        eprintln!("Error listing files: {}", e);
      },
    }
    return;
  }

  // Read and display specific file
  let file_path = &args[1];

  match read_snapshot_file(file_path) {
    Ok(snapshot) => {
      display_snapshot(&snapshot);

      // Additional analysis
      analyze_snapshot(&snapshot);
    },
    Err(e) => {
      eprintln!("Error reading snapshot file: {}", e);
      eprintln!("Make sure the file exists and was created by ttlog");
    },
  }
}

fn analyze_snapshot(snapshot: &Snapshot) {
  println!("\n=== ANALYSIS ===");

  // Count events by level
  let mut level_counts = std::collections::HashMap::new();
  for event in &snapshot.events {
    *level_counts.entry(&event.level).or_insert(0) += 1;
  }

  println!("Events by level:");
  for (level, count) in level_counts {
    println!("  {}: {}", level, count);
  }

  // Find time range
  if !snapshot.events.is_empty() {
    let min_time = snapshot.events.iter().map(|e| e.timestamp).min().unwrap();
    let max_time = snapshot.events.iter().map(|e| e.timestamp).max().unwrap();
    let duration_ms = max_time - min_time;

    println!("Time range: {} ms", duration_ms);
    println!("First event: {}", format_timestamp(min_time));
    println!("Last event: {}", format_timestamp(max_time));
  }

  // Count unique targets
  let unique_targets: std::collections::HashSet<_> =
    snapshot.events.iter().map(|e| &e.target).collect();
  println!("Unique targets: {}", unique_targets.len());

  // Show targets
  println!("Targets:");
  for target in unique_targets {
    println!("  {}", target);
  }
}

// Helper function for other programs to programmatically read snapshots
pub fn read_latest_snapshot() -> Result<Snapshot, Box<dyn std::error::Error>> {
  let files = list_snapshot_files()?;
  if files.is_empty() {
    return Err("No snapshot files found".into());
  }

  // Get the most recent file (they're sorted)
  let latest_file = files.last().unwrap();
  read_snapshot_file(latest_file)
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::fs;

  #[test]
  fn test_can_list_snapshot_files() {
    // This test will pass if there are any snapshot files
    // or if the /tmp directory is accessible
    match list_snapshot_files() {
      Ok(_files) => {
        // Test passes if we can list files (even if empty)
        assert!(true);
      },
      Err(e) => {
        panic!("Could not list snapshot files: {}", e);
      },
    }
  }
}
