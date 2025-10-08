use serde::{Deserialize, Serialize};
use std::path::Path;
use std::{
  fs,
  io::{self, BufRead},
  sync::{Arc, Mutex},
  collections::HashMap,
  time::Instant,
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
  /// Reads and parses log events from a log file (cached version with streaming support)
  ///
  /// # Arguments
  /// * `path` - Path to the .log file to read
  ///
  /// # Returns
  /// Vector of resolved log events
  pub fn get_logs(&self, path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    let cache = &self.cache;
    let mut cache_guard = cache.lock().unwrap();
    
    if let Some(cached_logs) = cache_guard.get(path) {
      return Ok(cached_logs.clone());
    }
    
    // Load from file with optimized reading
    let logs = Self::load_logs_from_file_optimized(path)?;
    cache_guard.insert(path.to_string(), logs.clone());
    
    Ok(logs)
  }
  
  /// Fast streaming log loader with lazy evaluation
  pub fn get_logs_streaming(&self, path: &str, limit: Option<usize>) -> Result<Vec<ResolvedLog>, io::Error> {
    // For streaming, we don't cache to avoid memory bloat
    Self::load_logs_streaming(path, limit)
  }

  /// Static version for backward compatibility
  pub fn get_logs_static(path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    Self::instance().get_logs(path)
  }

  // Legacy method - kept for compatibility
  fn load_logs_from_file(path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    Self::load_logs_from_file_optimized(path)
  }
  
  // Optimized log file loading with performance profiling
  fn load_logs_from_file_optimized(path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    let start_time = Instant::now();
    
    // Check file size first to choose optimal strategy
    let file_metadata = fs::metadata(path)?;
    let file_size = file_metadata.len();
    
    // For very large files (>50MB), use streaming approach
    if file_size > 50 * 1024 * 1024 {
      eprintln!("[PERF] Large file detected ({:.1}MB), using streaming approach", file_size as f64 / 1024.0 / 1024.0);
      return Self::load_logs_streaming_optimized(path);
    }
    
    use std::io::Read;
    
    // Read entire file into memory at once (much faster than line-by-line)
    let read_start = Instant::now();
    let mut file = fs::File::open(path)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    eprintln!("[PERF] File read took: {:.2}ms", read_start.elapsed().as_secs_f64() * 1000.0);
    
    // Pre-allocate vector with estimated capacity to reduce reallocations
    let estimated_lines = contents.matches('\n').count();
    let mut events: Vec<ResolvedLog> = Vec::with_capacity(estimated_lines);
    
    // Process lines in batches for better performance
    let parse_start = Instant::now();
    let lines: Vec<&str> = contents.lines().collect();
    eprintln!("[PERF] Line splitting took: {:.2}ms for {} lines", parse_start.elapsed().as_secs_f64() * 1000.0, lines.len());
    
    let process_start = Instant::now();
    let result = if lines.len() > 5000 {
      eprintln!("[PERF] Using parallel processing for {} lines", lines.len());
      Self::load_logs_parallel_optimized(&lines)
    } else if lines.len() > 1000 {
      eprintln!("[PERF] Using batch processing for {} lines", lines.len());
      Self::load_logs_batch_optimized(&lines)
    } else {
      eprintln!("[PERF] Using sequential processing for {} lines", lines.len());
      Self::load_logs_sequential_optimized(&lines)
    };
    
    eprintln!("[PERF] Processing took: {:.2}ms", process_start.elapsed().as_secs_f64() * 1000.0);
    eprintln!("[PERF] Total load time: {:.2}ms", start_time.elapsed().as_secs_f64() * 1000.0);
    
    result
  }
  
  // Optimized sequential processing with detailed profiling
  fn load_logs_sequential_optimized(lines: &[&str]) -> Result<Vec<ResolvedLog>, io::Error> {
    let start_time = Instant::now();
    let mut events = Vec::with_capacity(lines.len());
    
    // Profile different stages
    let mut json_parse_time = 0.0;
    let mut transform_time = 0.0;
    let mut processed_lines = 0;
    
    for line in lines {
      // Skip empty lines early
      if line.is_empty() {
        continue;
      }
      
      let line_start = Instant::now();
      
      // Test if JSON parsing is the bottleneck
      let parse_start = Instant::now();
      if let Some(event) = Self::parse_log_line_fast(line) {
        json_parse_time += parse_start.elapsed().as_secs_f64();
        
        let transform_start = Instant::now();
        events.push(event);
        transform_time += transform_start.elapsed().as_secs_f64();
      } else {
        json_parse_time += parse_start.elapsed().as_secs_f64();
      }
      
      processed_lines += 1;
      
      // Sample profiling every 1000 lines to avoid overhead
      if processed_lines % 1000 == 0 {
        let elapsed = start_time.elapsed().as_secs_f64();
        eprintln!("[PERF] Processed {} lines in {:.2}s (avg: {:.3}ms/line)", 
                 processed_lines, elapsed, (elapsed * 1000.0) / processed_lines as f64);
        eprintln!("[PERF] JSON parse time: {:.2}s, Transform time: {:.2}s", json_parse_time, transform_time);
      }
    }
    
    let total_time = start_time.elapsed().as_secs_f64();
    eprintln!("[PERF] Sequential processing complete: {} lines in {:.2}s", processed_lines, total_time);
    eprintln!("[PERF] JSON parsing took: {:.2}s ({:.1}%)", json_parse_time, (json_parse_time / total_time) * 100.0);
    eprintln!("[PERF] Transform took: {:.2}s ({:.1}%)", transform_time, (transform_time / total_time) * 100.0);
    
    // Shrink vector to actual size to free unused memory
    events.shrink_to_fit();
    Ok(events)
  }
  
  // Highly optimized parallel processing for large log files
  fn load_logs_parallel_optimized(lines: &[&str]) -> Result<Vec<ResolvedLog>, io::Error> {
    use std::sync::mpsc;
    use std::thread;
    
    // Dynamic chunk size based on line count for optimal performance
    let chunk_size = if lines.len() > 100000 {
      10000 // Larger chunks for very large files
    } else if lines.len() > 50000 {
      5000  // Medium chunks for large files
    } else {
      2000  // Smaller chunks for moderate files
    };
    
    let num_threads = std::thread::available_parallelism().map(|n| n.get()).unwrap_or(4);
    eprintln!("[PERF] Using {} threads with chunk size {}", num_threads, chunk_size);
    let (tx, rx) = mpsc::channel();
    let mut handles = Vec::new();
    
    // Split work across multiple threads
    for chunk in lines.chunks(chunk_size) {
      let tx = tx.clone();
      let chunk: Vec<String> = chunk.iter().map(|s| s.to_string()).collect();
      
      let handle = thread::spawn(move || {
        let mut chunk_events = Vec::with_capacity(chunk.len());
        
        for line in chunk {
          if line.is_empty() {
            continue;
          }
          
          if let Some(event) = Self::parse_log_line_fast(&line) {
            chunk_events.push(event);
          }
        }
        
        tx.send(chunk_events).unwrap();
      });
      
      handles.push(handle);
    }
    
    drop(tx); // Close sender
    
    // Collect results from all threads
    let mut all_events = Vec::new();
    for chunk_events in rx {
      all_events.extend(chunk_events);
    }
    
    // Wait for all threads to complete
    for handle in handles {
      handle.join().unwrap();
    }
    
    all_events.shrink_to_fit();
    Ok(all_events)
  }
  
  // Optimized batch processing for medium-sized files
  fn load_logs_batch_optimized(lines: &[&str]) -> Result<Vec<ResolvedLog>, io::Error> {
    let mut events = Vec::with_capacity(lines.len());
    
    // Process in batches to reduce allocations
    const BATCH_SIZE: usize = 1000;
    
    for batch in lines.chunks(BATCH_SIZE) {
      let mut batch_events = Vec::with_capacity(BATCH_SIZE);
      
      for line in batch {
        if line.is_empty() {
          continue;
        }
        
        if let Some(event) = Self::parse_log_line_fast(line) {
          batch_events.push(event);
        }
      }
      
      events.extend(batch_events);
    }
    
    events.shrink_to_fit();
    Ok(events)
  }
  
  // Optimized streaming loader for very large files
  fn load_logs_streaming_optimized(path: &str) -> Result<Vec<ResolvedLog>, io::Error> {
    let start_time = Instant::now();
    use std::io::{BufRead, BufReader};
    
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(256 * 1024, file); // 256KB buffer for large files
    let mut events = Vec::new();
    let mut processed_lines = 0;
    
    for line in reader.lines() {
      let line = line?;
      
      if line.is_empty() {
        continue;
      }
      
      if let Some(event) = Self::parse_log_line_fast(&line) {
        events.push(event);
      }
      
      processed_lines += 1;
      
      // Progress reporting for very large files
      if processed_lines % 50000 == 0 {
        eprintln!("[PERF] Processed {} lines in {:.1}s", processed_lines, start_time.elapsed().as_secs_f64());
      }
    }
    
    events.shrink_to_fit();
    eprintln!("[PERF] Streaming completed: {} lines in {:.2}s", processed_lines, start_time.elapsed().as_secs_f64());
    Ok(events)
  }
  
  // Legacy streaming method with limit support
  fn load_logs_streaming(path: &str, limit: Option<usize>) -> Result<Vec<ResolvedLog>, io::Error> {
    use std::io::{BufRead, BufReader};
    
    let file = fs::File::open(path)?;
    let reader = BufReader::with_capacity(64 * 1024, file); // 64KB buffer
    let mut events = Vec::new();
    let mut count = 0;
    
    for line in reader.lines() {
      let line = line?;
      
      if line.is_empty() {
        continue;
      }
      
      if let Some(event) = Self::parse_log_line_fast(&line) {
        events.push(event);
        count += 1;
        
        // Stop if we've reached the limit
        if let Some(max_count) = limit {
          if count >= max_count {
            break;
          }
        }
      }
    }
    
    events.shrink_to_fit();
    Ok(events)
  }
  
  // Ultra-optimized log line parsing with detailed profiling
  fn parse_log_line_fast(line: &str) -> Option<ResolvedLog> {
    // Skip JSON parsing profiling for individual lines (too verbose)
    // Focus on the actual bottlenecks
    
    // Fast JSON parsing - this might be the bottleneck!
    let parse_result = serde_json::from_str::<LogFileEvent>(line);
    
    match parse_result {
      Ok(log_event) => {
        // Timestamp formatting might be expensive
        let timestamp = Utils::format_timestamp(log_event.timestamp);
        
        // Creating the ResolvedLog struct
        Some(ResolvedLog {
          level: LogLevel::from_u8(&log_event.level),
          timestamp,
          thread_id: log_event.thread_id,
          message: log_event.message,
          target: log_event.target,
          kv: log_event.kv,
          file: log_event.file,
          position: log_event.position,
        })
      },
      Err(_) => None,
    }
  }
  
  // Minimal parsing for profiling - skip expensive operations
  fn parse_log_line_minimal(line: &str) -> Option<()> {
    // Just test JSON parsing speed without creating full objects
    match serde_json::from_str::<LogFileEvent>(line) {
      Ok(_) => Some(()),
      Err(_) => None,
    }
  }

  /// Gets detailed information about log files in a directory (cached version)
  ///
  /// # Arguments
  /// * `path` - Path to the directory containing log files
  ///
  /// # Returns
  /// LogsInfo struct containing statistics about all log files
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

  /// Gets both log content and directory information (cached version)
  ///
  /// # Arguments
  /// * `dir_path` - Path to the directory containing log files
  /// * `log_file_path` - Path to the specific .log file to read
  ///
  /// # Returns
  /// Tuple of (LogsInfo, Vec<ResolvedLog>)
  pub fn get_complete_logs_data(
    &self,
    dir_path: &str,
    log_file_path: &str,
  ) -> Result<(LogsInfo, Vec<ResolvedLog>), io::Error> {
    let logs_info = self.get_logs_info(dir_path)?;
    let log_events = self.get_logs(log_file_path)?;
    Ok((logs_info, log_events))
  }

  /// Static version for backward compatibility
  pub fn get_complete_logs_data_static(
    dir_path: &str,
    log_file_path: &str,
  ) -> Result<(LogsInfo, Vec<ResolvedLog>), io::Error> {
    Self::instance().get_complete_logs_data(dir_path, log_file_path)
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
        read_directory(&entry_path, total_size, total_files, bin_files, log_files, total_lines, total_events)?;
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
