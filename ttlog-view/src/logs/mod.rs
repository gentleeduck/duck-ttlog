use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
  collections::HashMap,
  fs,
  io::{self},
  sync::{Arc, Mutex},
};
use ttlog::event::LogLevel;

use crate::utils::Utils;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogsInfo {
  pub total_size: u64,
  pub total_size_formatted: String,
  pub total_files: usize,
  pub bin_files: BinFilesInfo,
  pub log_files: LogFilesInfo,
  pub directory_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BinFilesInfo {
  pub count: usize,
  pub total_size: u64,
  pub total_size_formatted: String,
  pub min_size: Option<u64>,
  pub min_size_formatted: Option<String>,
  pub max_size: Option<u64>,
  pub max_size_formatted: Option<String>,
  pub avg_size: u64,
  pub avg_size_formatted: String,
  pub files: Vec<FileInfo>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFilesInfo {
  pub count: usize,
  pub total_size: u64,
  pub total_size_formatted: String,
  pub total_lines: usize,
  pub total_events: usize,
  pub files: Vec<FileInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
  pub name: String,
  pub size: u64,
  pub size_formatted: String,
  pub timestamp: Option<String>,
  pub process_id: Option<String>,
  pub line_count: Option<usize>,
  pub event_count: Option<usize>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogFileEvent {
  pub file: String,
  pub kv: serde_json::Value,
  pub level: u8,
  pub message: String,
  pub position: (u32, u32),
  pub target: String,
  pub thread_id: u8,
  pub timestamp: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ResolvedLog {
  pub level: LogLevel,
  pub timestamp: String,
  pub thread_id: u8,
  pub message: String,
  pub target: String,
  pub kv: serde_json::Value,
  pub file: String,
  pub position: (u32, u32),
}

pub struct Logs {
  cache: Arc<Mutex<HashMap<String, Vec<ResolvedLog>>>>,
  info_cache: Arc<Mutex<HashMap<String, LogsInfo>>>,
}

static LOGS_INSTANCE: std::sync::OnceLock<Logs> = std::sync::OnceLock::new();

impl Logs {
  pub fn instance() -> &'static Logs {
    LOGS_INSTANCE.get_or_init(|| Logs {
      cache: Arc::new(Mutex::new(HashMap::new())),
      info_cache: Arc::new(Mutex::new(HashMap::new())),
    })
  }

  pub fn clear_cache(&self) {
    if let Ok(mut cache) = self.cache.lock() {
      cache.clear();
    }
    if let Ok(mut info_cache) = self.info_cache.lock() {
      info_cache.clear();
    }
  }

  pub fn get_logs_info(&self, path: &str) -> Result<LogsInfo, io::Error> {
    // Check cache first
    if let Ok(cache) = self.info_cache.lock() {
      if let Some(cached_info) = cache.get(path) {
        return Ok(cached_info.clone());
      }
    }

    // Load from directory
    let info = Self::load_logs_info_from_dir(path)?;

    // Cache the result
    if let Ok(mut cache) = self.info_cache.lock() {
      cache.insert(path.to_string(), info.clone());
    }

    Ok(info)
  }

  /// Static version for backward compatibility
  pub fn get_logs_info_static(path: &str) -> Result<LogsInfo, io::Error> {
    Self::instance().get_logs_info(path)
  }

  fn load_logs_info_from_dir(path: &str) -> Result<LogsInfo, io::Error> {
    let path_obj = Path::new(path);

    if !path_obj.exists() {
      return Err(io::Error::new(
        io::ErrorKind::NotFound,
        format!("Path does not exist: {}", path_obj.display()),
      ));
    }

    let mut total_size = 0u64;
    let mut total_files = 0usize;
    let mut bin_files = Vec::new();
    let mut log_files = Vec::new();
    let mut total_lines = 0usize;
    let mut total_events = 0usize;

    // Read directory recursively
    read_directory(
      path_obj,
      &mut total_size,
      &mut total_files,
      &mut bin_files,
      &mut log_files,
      &mut total_lines,
      &mut total_events,
    )?;

    // Calculate bin files statistics
    let bin_total_size: u64 = bin_files.iter().map(|f| f.size).sum();
    let bin_count = bin_files.len();
    let bin_min_size = bin_files.iter().map(|f| f.size).min();
    let bin_max_size = bin_files.iter().map(|f| f.size).max();
    let bin_avg_size = if bin_count > 0 {
      bin_total_size / bin_count as u64
    } else {
      0
    };

    // Calculate log files statistics
    let log_total_size: u64 = log_files.iter().map(|f| f.size).sum();
    let log_count = log_files.len();

    Ok(LogsInfo {
      total_size,
      total_size_formatted: Utils::format_size(total_size),
      total_files,
      directory_path: path.to_string(),
      bin_files: BinFilesInfo {
        count: bin_count,
        total_size: bin_total_size,
        total_size_formatted: Utils::format_size(bin_total_size),
        min_size: bin_min_size,
        min_size_formatted: bin_min_size.map(Utils::format_size),
        max_size: bin_max_size,
        max_size_formatted: bin_max_size.map(Utils::format_size),
        avg_size: bin_avg_size,
        avg_size_formatted: Utils::format_size(bin_avg_size),
        files: bin_files,
      },
      log_files: LogFilesInfo {
        count: log_count,
        total_size: log_total_size,
        total_size_formatted: Utils::format_size(log_total_size),
        total_lines,
        total_events,
        files: log_files,
      },
    })
  }
}

/// Recursively reads a directory and collects file information
fn read_directory(
  path: &Path,
  total_size: &mut u64,
  total_files: &mut usize,
  bin_files: &mut Vec<FileInfo>,
  log_files: &mut Vec<FileInfo>,
  total_lines: &mut usize,
  total_events: &mut usize,
) -> Result<(), io::Error> {
  if path.is_dir() {
    for entry in fs::read_dir(path)? {
      let entry = entry?;
      let entry_path = entry.path();

      if entry_path.is_dir() {
        read_directory(
          &entry_path,
          total_size,
          total_files,
          bin_files,
          log_files,
          total_lines,
          total_events,
        )?;
      } else if entry_path.is_file() {
        if let Ok(metadata) = fs::metadata(&entry_path) {
          let size = metadata.len();
          *total_size += size;
          *total_files += 1;

          let file_name = entry_path
            .file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("")
            .to_string();

          // Extract timestamp and process ID from filename
          let (timestamp, process_id) = extract_metadata(&file_name);

          // Count lines and events for .log files
          let (line_count, event_count) = if file_name.ends_with(".log") {
            match count_log_file_stats(&entry.path()) {
              Ok((lines, events)) => {
                *total_lines += lines;
                *total_events += events;
                (Some(lines), Some(events))
              },
              Err(_) => (None, None),
            }
          } else {
            (None, None)
          };

          let file_info = FileInfo {
            name: file_name.clone(),
            size,
            size_formatted: Utils::format_size(size),
            timestamp,
            process_id,
            line_count,
            event_count,
          };

          if file_name.ends_with(".bin") {
            bin_files.push(file_info);
          } else if file_name.ends_with(".log") {
            log_files.push(file_info);
          }
        }
      }
    }
  }
  Ok(())
}

/// Counts lines and events in a log file efficiently
fn count_log_file_stats(path: &Path) -> Result<(usize, usize), io::Error> {
  use std::io::{BufRead, BufReader};

  let file = fs::File::open(path)?;
  let reader = BufReader::with_capacity(64 * 1024, file);
  let mut line_count = 0;
  let mut event_count = 0;

  for line in reader.lines() {
    let line = line?;
    line_count += 1;

    // Skip empty lines
    if line.trim().is_empty() {
      continue;
    }

    // Try to parse as JSON to count valid events
    if serde_json::from_str::<LogFileEvent>(&line).is_ok() {
      event_count += 1;
    }
  }

  Ok((line_count, event_count))
}

/// Extracts metadata (timestamp and process ID) from a filename
///
/// Expected format: ttlog-{PID}-{TIMESTAMP}-{SUFFIX}.{EXT}
/// Example: ttlog-1580331-20251007215735-flush_and_exit.bin
fn extract_metadata(filename: &str) -> (Option<String>, Option<String>) {
  let parts: Vec<&str> = filename.split('-').collect();

  let process_id = if parts.len() >= 2 && parts[0] == "ttlog" {
    Some(parts[1].to_string())
  } else {
    None
  };

  let timestamp = if parts.len() >= 3 {
    let timestamp_str = parts[2];
    if timestamp_str.len() == 14 {
      // Format: YYYYMMDDHHMMSS
      let year = &timestamp_str[0..4];
      let month = &timestamp_str[4..6];
      let day = &timestamp_str[6..8];
      let hour = &timestamp_str[8..10];
      let minute = &timestamp_str[10..12];
      let second = &timestamp_str[12..14];
      Some(format!(
        "{}-{}-{} {}:{}:{}",
        year, month, day, hour, minute, second
      ))
    } else {
      None
    }
  } else {
    None
  };

  (timestamp, process_id)
}
