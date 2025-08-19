mod __test__;

use chrono::Utc;
use lz4::block::{compress, CompressionMode};
use serde::{Deserialize, Serialize};
use std::fs::{self, File};
use std::io::Write;

use crate::event::LogEvent;
use crate::lf_buffer::LockFreeRingBuffer as RingBuffer;

/// A snapshot bundles metadata together with a sequence of events.
///
/// Snapshots provide a point-in-time capture of logging events along with
/// contextual metadata about the system state when the snapshot was created.
/// This enables comprehensive debugging, monitoring, and audit trails.
///
/// # Purpose
///
/// Snapshots serve multiple purposes:
/// - **Debugging**: Capture system state during errors or anomalies
/// - **Monitoring**: Periodic captures for performance analysis
/// - **Compliance**: Audit trails with complete context
/// - **Forensics**: Post-incident analysis with full event history
///
/// # Metadata Fields
///
/// Each snapshot includes rich metadata to provide context:
/// - **Service identification**: Service name for multi-service environments
/// - **System context**: Hostname and process ID for deployment tracking
/// - **Temporal context**: Creation timestamp for chronological ordering
/// - **Causation context**: Reason for snapshot creation (manual, error, periodic, etc.)
///
/// # Serialization Support
///
/// Snapshots can be serialized to various formats (JSON, CBOR, etc.) for:
/// - Persistent storage
/// - Network transmission
/// - Integration with external systems
/// - Long-term archival
///
/// # Example
/// ```rust,ignore
/// let snapshot = Snapshot {
///     service: "auth-service".to_string(),
///     hostname: "web-01".to_string(),
///     pid: 12345,
///     created_at: "20230819123000".to_string(),
///     reason: "error-threshold-exceeded".to_string(),
///     events: vec![/* log events */],
/// };
/// ```
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Snapshot {
  /// Service name identifier.
  ///
  /// Used to distinguish snapshots from different services in multi-service
  /// environments. Should be consistent across all instances of the same service.
  pub service: String,

  /// Hostname where the snapshot was created.
  ///
  /// Provides deployment context, enabling correlation of events across
  /// different machines in distributed systems. Automatically populated
  /// from the system hostname.
  pub hostname: String,

  /// Process ID of the service that created the snapshot.
  ///
  /// Enables distinction between multiple instances of the same service
  /// running on the same host. Useful for debugging process-specific issues.
  pub pid: u32,

  /// Timestamp when the snapshot was created.
  ///
  /// Format: `YYYYMMDDHHMMSS` (e.g., "20230819123000" for Aug 19, 2023 12:30:00 UTC).
  /// This compact format is both human-readable and suitable for filename generation.
  pub created_at: String,

  /// Reason why the snapshot was created.
  ///
  /// Provides context for snapshot creation, enabling categorization and analysis.
  /// Common values:
  /// - "manual": User-initiated snapshot
  /// - "error": Triggered by error conditions
  /// - "periodic": Scheduled/automatic snapshot
  /// - "shutdown": Service shutdown snapshot
  /// - "memory-pressure": High memory usage trigger
  pub reason: String,

  /// The captured log events in chronological order.
  ///
  /// Events are stored in FIFO order (oldest first) as captured from
  /// the ring buffer. May represent the entire buffer contents or a
  /// filtered subset depending on the snapshot strategy.
  pub events: Vec<LogEvent>,
}

/// Writer for creating and persisting log event snapshots.
///
/// The `SnapshotWriter` provides a high-level interface for capturing log events
/// from ring buffers and persisting them to disk with compression and atomic writes.
/// It handles all the complexity of serialization, compression, and safe file operations.
///
/// # Features
///
/// - **Atomic writes**: Uses temporary files + rename for crash safety
/// - **Compression**: LZ4 compression for efficient storage
/// - **CBOR serialization**: Fast, compact binary format
/// - **Rich metadata**: Automatic collection of system context
/// - **Error handling**: Comprehensive error propagation
///
/// # File Format
///
/// Written files contain:
/// 1. LZ4-compressed CBOR-serialized `Snapshot` data
/// 2. Filename pattern: `/tmp/ttlog-<pid>-<timestamp>-<reason>.bin`
///
/// # Thread Safety
///
/// `SnapshotWriter` is thread-safe and can be shared across threads.
/// Multiple threads can create snapshots concurrently without coordination.
///
/// # Example
/// ```rust,ignore
/// use ttlog::snapshot::SnapshotWriter;
/// use ttlog::lf_buffer::LockFreeRingBuffer;
///
/// let writer = SnapshotWriter::new("auth-service");
/// let mut ring = LockFreeRingBuffer::new(1000);
///
/// // ... populate ring with events ...
///
/// // Create and write snapshot
/// writer.snapshot_and_write(&mut ring, "periodic-backup")?;
/// ```
#[derive(Debug, Clone)]
pub struct SnapshotWriter {
  /// Service name used for snapshot metadata.
  ///
  /// This identifier is included in all snapshots created by this writer,
  /// enabling service-level filtering and organization of snapshot files.
  service: String,
}

impl SnapshotWriter {
  /// Creates a new snapshot writer for the specified service.
  ///
  /// # Arguments
  /// * `service` - Service name identifier. This will be included in all
  ///               snapshots created by this writer.
  ///
  /// # Example
  /// ```rust,ignore
  /// let writer = SnapshotWriter::new("user-auth-service");
  /// let writer2 = SnapshotWriter::new(String::from("payment-processor"));
  /// ```
  pub fn new(service: impl Into<String>) -> Self {
    Self {
      service: service.into(),
    }
  }

  /// Creates a snapshot from the current state of a ring buffer.
  ///
  /// This method captures all events currently in the ring buffer along with
  /// rich metadata about the system state. The ring buffer is drained during
  /// this process (via `take_snapshot()`), so events are consumed.
  ///
  /// # Arguments
  /// * `ring` - Mutable reference to the ring buffer to snapshot
  /// * `reason` - Contextual reason for creating this snapshot
  ///
  /// # Returns
  /// * `Some(Snapshot)` - Successfully created snapshot with events and metadata
  /// * `None` - Ring buffer was empty, no snapshot created
  ///
  /// # Metadata Collection
  ///
  /// The method automatically collects:
  /// - **Hostname**: Via `gethostname::gethostname()`
  /// - **Process ID**: Via `std::process::id()`
  /// - **Timestamp**: Current UTC time in `YYYYMMDDHHMMSS` format
  /// - **Service name**: From the writer's configuration
  ///
  /// # Performance
  /// - **Time**: O(n) where n is the number of events in the buffer
  /// - **Memory**: Creates Vec<LogEvent> to hold all events
  /// - **I/O**: No I/O operations, only in-memory processing
  ///
  /// # Example
  /// ```rust,ignore
  /// let writer = SnapshotWriter::new("api-gateway");
  /// let mut ring = RingBuffer::new(500);
  ///
  /// // Add some events to the ring
  /// ring.push_overwrite(create_log_event("User login"));
  /// ring.push_overwrite(create_log_event("Database query"));
  ///
  /// // Create snapshot
  /// if let Some(snapshot) = writer.create_snapshot(&mut ring, "manual-debug") {
  ///     println!("Captured {} events", snapshot.events.len());
  ///     println!("Reason: {}", snapshot.reason);
  /// }
  /// ```
  pub fn create_snapshot(
    &self,
    ring: &mut RingBuffer<LogEvent>,
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
  /// This method implements a robust file writing strategy that ensures data
  /// integrity even in the face of system crashes or power failures.
  ///
  /// # Process Overview
  ///
  /// The writing process follows these steps:
  /// 1. **Serialize**: Convert `Snapshot` to CBOR (Concise Binary Object Representation)
  /// 2. **Compress**: Apply LZ4 compression for space efficiency
  /// 3. **Atomic Write**: Write to temporary file + rename for crash safety
  /// 4. **Verification**: Sync to disk before finalizing
  ///
  /// # Arguments
  /// * `snapshot` - The snapshot to write to disk
  ///
  /// # Returns
  /// * `Ok(())` - Snapshot successfully written and synced to disk
  /// * `Err(Box<dyn std::error::Error>)` - Any step in the process failed
  ///
  /// # File Naming Convention
  ///
  /// Files are written to: `/tmp/ttlog-<pid>-<created_at>-<reason>.bin`
  ///
  /// Example: `/tmp/ttlog-12345-20230819123000-error.bin`
  ///
  /// This naming scheme provides:
  /// - **Uniqueness**: PID + timestamp prevent collisions
  /// - **Sorting**: Lexicographic sorting gives chronological order
  /// - **Context**: Reason is immediately visible in filename
  ///
  /// # Error Scenarios
  ///
  /// The method can fail at several points:
  /// - **Serialization errors**: Malformed data structures
  /// - **Compression errors**: LZ4 internal failures (rare)
  /// - **I/O errors**: Disk full, permissions, network filesystems
  /// - **Atomic operation errors**: Filesystem doesn't support atomic rename
  ///
  /// # Atomicity Guarantees
  ///
  /// The write-to-temp-then-rename pattern ensures:
  /// - **Crash safety**: Incomplete writes don't corrupt existing files
  /// - **Consistency**: Readers never see partially written snapshots
  /// - **Durability**: `sync_all()` ensures data reaches persistent storage
  ///
  /// # Storage Efficiency
  ///
  /// LZ4 compression typically achieves:
  /// - **Text logs**: 60-80% size reduction
  /// - **Structured data**: 40-70% size reduction
  /// - **Repeated patterns**: Up to 90% size reduction
  /// - **Compression speed**: ~200-400 MB/s
  ///
  /// # Example
  /// ```rust,ignore
  /// let writer = SnapshotWriter::new("payment-service");
  /// let snapshot = Snapshot {
  ///     service: "payment-service".to_string(),
  ///     hostname: "prod-01".to_string(),
  ///     pid: 54321,
  ///     created_at: "20230819140000".to_string(),
  ///     reason: "high-error-rate".to_string(),
  ///     events: captured_events,
  /// };
  ///
  /// match writer.write_snapshot(&snapshot) {
  ///     Ok(()) => println!("Snapshot saved successfully"),
  ///     Err(e) => eprintln!("Failed to save snapshot: {}", e),
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

  /// Convenience method: create a snapshot from `ring` and write it to disk if non-empty.
  ///
  /// This method combines `create_snapshot()` and `write_snapshot()` into a single
  /// operation, handling the common case where you want to immediately persist
  /// a snapshot without intermediate processing.
  ///
  /// # Process Flow
  ///
  /// 1. **Capture**: Extract events from ring buffer with metadata
  /// 2. **Validate**: Check if any events were captured
  /// 3. **Persist**: Write to disk if events exist
  /// 4. **Report**: Provide user feedback on the operation
  ///
  /// # Arguments
  /// * `ring` - Mutable reference to the ring buffer to snapshot
  /// * `reason` - Contextual reason for creating this snapshot
  ///
  /// # Returns
  /// * `Ok(())` - Snapshot created and written successfully, or no events to snapshot
  /// * `Err(Box<dyn std::error::Error>)` - Snapshot creation or writing failed
  ///
  /// # Behavior for Empty Buffers
  ///
  /// If the ring buffer is empty:
  /// - No snapshot file is created
  /// - A message is printed to stdout
  /// - `Ok(())` is returned (not an error condition)
  ///
  /// # Use Cases
  ///
  /// This method is ideal for:
  /// - **Periodic snapshots**: Scheduled captures via cron or timers
  /// - **Error-triggered snapshots**: Automatic capture during exceptions
  /// - **Manual debugging**: Interactive snapshot creation
  /// - **Shutdown procedures**: Final state capture before service termination
  ///
  /// # Example Usage Patterns
  /// ```rust,ignore
  /// let writer = SnapshotWriter::new("user-service");
  /// let mut ring = RingBuffer::new(2000);
  ///
  /// // Periodic snapshot (e.g., every 5 minutes)
  /// std::thread::spawn(move || {
  ///     loop {
  ///         std::thread::sleep(std::time::Duration::from_secs(300));
  ///         if let Err(e) = writer.snapshot_and_write(&mut ring, "periodic") {
  ///             eprintln!("Periodic snapshot failed: {}", e);
  ///         }
  ///     }
  /// });
  ///
  /// // Error-triggered snapshot
  /// if error_count > threshold {
  ///     writer.snapshot_and_write(&mut ring, "error-threshold-exceeded")?;
  /// }
  ///
  /// // Manual debugging snapshot
  /// writer.snapshot_and_write(&mut ring, "manual-debug")?;
  ///
  /// // Graceful shutdown snapshot
  /// writer.snapshot_and_write(&mut ring, "shutdown")?;
  /// ```
  pub fn snapshot_and_write(
    &self,
    ring: &mut RingBuffer<LogEvent>,
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
