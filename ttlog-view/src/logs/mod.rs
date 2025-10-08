use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
  fs,
  io::{self, BufRead},
};
use ttlog::event::LogLevel;

use crate::utils::Utils;

#[derive(Debug, Serialize, Deserialize)]
pub struct LogsInfo {
  pub total_size: u64,
  pub total_size_formatted: String,
  pub total_files: usize,
  pub bin_files: BinFilesInfo,
  pub log_files: LogFilesInfo,
  pub directory_path: String,
}

#[derive(Debug, Serialize, Deserialize)]
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

#[derive(Debug, Serialize, Deserialize)]
pub struct LogFilesInfo {
  pub count: usize,
  pub total_size: u64,
  pub total_size_formatted: String,
  pub files: Vec<FileInfo>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FileInfo {
  pub name: String,
  pub size: u64,
  pub size_formatted: String,
  pub timestamp: Option<String>,
  pub process_id: Option<String>,
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

pub struct Logs;

impl Logs {
  /// Reads and parses log events from a log file
  ///
  /// # Arguments
  /// * `path` - Path to the .log file to read
  ///
  /// # Returns
  /// Vector of resolved log events
  pub fn get_logs(path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    let file = fs::File::open(path)?;
    let reader = io::BufReader::new(file);
    let mut events: Vec<ResolvedLog> = Vec::new();

    for line in reader.lines() {
      let line = line?;

      // Skip empty lines
      if line.trim().is_empty() {
        continue;
      }

      // Deserialize the line as a LogFileEvent
      match serde_json::from_str::<LogFileEvent>(&line) {
        Ok(log_event) => {
          let level = LogLevel::from_u8(&log_event.level);
          let timestamp = Utils::format_timestamp(log_event.timestamp);

          let event = ResolvedLog {
            level,
            timestamp,
            thread_id: log_event.thread_id,
            message: log_event.message,
            target: log_event.target,
            kv: log_event.kv,
            file: log_event.file,
            position: log_event.position,
          };

          events.push(event);
        },
        Err(e) => {
          eprintln!("Warning: Failed to parse log line: {}", e);
          continue;
        },
      }
    }

    Ok(events)
  }

  /// Gets detailed information about log files in a directory
  ///
  /// # Arguments
  /// * `path` - Path to the directory containing log files
  ///
  /// # Returns
  /// LogsInfo struct containing statistics about all log files
  pub fn get_logs_info(path: &str) -> Result<LogsInfo, io::Error> {
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

    // Read directory recursively
    read_directory(
      path_obj,
      &mut total_size,
      &mut total_files,
      &mut bin_files,
      &mut log_files,
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
        files: log_files,
      },
    })
  }

  /// Gets both log content and directory information
  ///
  /// # Arguments
  /// * `dir_path` - Path to the directory containing log files
  /// * `log_file_path` - Path to the specific .log file to read
  ///
  /// # Returns
  /// Tuple of (LogsInfo, Vec<ResolvedLog>)
  pub fn get_complete_logs_data(
    dir_path: &str,
    log_file_path: &str,
  ) -> Result<(LogsInfo, Vec<ResolvedLog>), io::Error> {
    let info = Self::get_logs_info(dir_path)?;
    let logs = Self::get_logs(log_file_path)?;
    Ok((info, logs))
  }
}

/// Recursively reads a directory and collects file information
fn read_directory(
  path: &Path,
  total_size: &mut u64,
  total_files: &mut usize,
  bin_files: &mut Vec<FileInfo>,
  log_files: &mut Vec<FileInfo>,
) -> Result<(), io::Error> {
  if path.is_dir() {
    for entry in fs::read_dir(path)? {
      let entry = entry?;
      let entry_path = entry.path();

      if entry_path.is_dir() {
        read_directory(&entry_path, total_size, total_files, bin_files, log_files)?;
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

          let file_info = FileInfo {
            name: file_name.clone(),
            size,
            size_formatted: Utils::format_size(size),
            timestamp,
            process_id,
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
