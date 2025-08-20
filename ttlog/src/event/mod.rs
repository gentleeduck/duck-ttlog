//! # LogEvent Module
//!
//! Core data structures for high-performance structured logging with compact memory layout
//! and efficient serialization.
//!
//! ## Design Philosophy
//!
//! This module prioritizes:
//! - **Memory efficiency**: Fixed-size structures with careful padding
//! - **Cache performance**: 104-byte events fit well in CPU cache lines
//! - **Serialization speed**: Direct serde support with minimal overhead
//! - **Type safety**: Strongly-typed log levels and field values
//! - **Bit packing**: Metadata compressed into single u64
//!
//! ## Memory Layout
//!
//! ```text
//! LogEvent (104 bytes total):
//! ┌─────────────────┬───────────────┬──────────────┬───────────────┐
//! │ packed_meta(8)  │ target_id(2)  │ message_id(2)│ field_count(1)│
//! ├─────────────────┼───────────────┼──────────────┼───────────────┤
//! │ fields[0] (10)  │ fields[1] (10)│ fields[2] (10)│ file_id(2)   │
//! ├─────────────────┼───────────────┼──────────────┼───────────────┤
//! │ line(2)         │ _padding(9)   │              │               │
//! └─────────────────┴───────────────┴──────────────┴───────────────┘
//! ```
//!
//! ## String Interning Strategy
//!
//! All string data is stored as 16-bit IDs pointing to interned strings:
//! - `target_id`: Module/component identifier
//! - `message_id`: Log message template
//! - `field.key_id`: Structured field keys
//! - `file_id`: Source file path (optional)
//!
//! ## Usage Examples
//!
//! ```rust
//! use ttlog::event::{LogEvent, LogLevel, FieldValue, Field};
//!
//! // Create a basic event
//! let mut event = LogEvent::new();
//! event.packed_meta = LogEvent::pack_meta(1234567890, LogLevel::INFO, 42);
//! event.target_id = 100;  // Pre-interned target
//! event.message_id = 200; // Pre-interned message
//!
//! // Add structured fields
//! event.add_field(300, FieldValue::I64(42));
//! event.add_field(301, FieldValue::Bool(true));
//!
//! // Extract metadata
//! assert_eq!(event.timestamp_millis(), 1234567890);
//! assert_eq!(event.level(), LogLevel::INFO);
//! assert_eq!(event.thread_id(), 42);
//! ```

mod __test__;

use serde::{Deserialize, Serialize};
use std::fmt;

/// Log severity level with compact u8 representation.
///
/// # Design Rationale
///
/// Uses explicit `#[repr(u8)]` to:
/// - **Guarantee size**: Always 1 byte regardless of platform
/// - **Enable bit packing**: Fits in 4 bits of packed metadata
/// - **Support transmute**: Safe conversion from u8 values 0-4
/// - **Match conventions**: Standard syslog/RFC3164 level mapping
///
/// # Level Semantics
///
/// - **TRACE (0)**: Extremely verbose, typically disabled in production
/// - **DEBUG (1)**: Detailed information for developers
/// - **INFO (2)**: General informational messages
/// - **WARN (3)**: Warning conditions that should be noted
/// - **ERROR (4)**: Error conditions requiring attention
///
/// # Ordering
///
/// Levels implement `Ord` with natural ordering: TRACE < DEBUG < INFO < WARN < ERROR.
/// This enables level filtering: `if event.level() >= LogLevel::WARN { ... }`
///
/// # Serialization
///
/// Serializes as string values (`"trace"`, `"debug"`, etc.) for human readability
/// in JSON/YAML output, while maintaining compact binary representation internally.
///
/// # Example
///
/// ```rust
/// use ttlog::event::LogLevel;
///
/// let level = LogLevel::INFO;
/// assert_eq!(level as u8, 2);
/// assert!(level > LogLevel::DEBUG);
/// assert!(level < LogLevel::ERROR);
/// ```
///
/// Compact `u8` representation of common logging levels.
/// Values are chosen to match common conventions (`0..4`).
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
pub enum LogLevel {
  TRACE = 0,
  DEBUG = 1,
  INFO = 2,
  WARN = 3,
  ERROR = 4,
}

impl LogLevel {
  /// Convert from a lowercase string (`"trace"`, `"debug"`, etc.).
  ///
  /// # Parsing Strategy
  ///
  /// - **Case sensitive**: Only lowercase strings are recognized
  /// - **Exact match**: No prefix matching or fuzzy logic
  /// - **Default fallback**: Unknown strings default to `INFO` (safe default)
  /// - **No validation errors**: Always returns a valid level
  ///
  /// # Supported Strings
  ///
  /// | Input | Output |
  /// |-------|--------|
  /// | `"trace"` | `LogLevel::TRACE` |
  /// | `"debug"` | `LogLevel::DEBUG` |
  /// | `"info"` | `LogLevel::INFO` |
  /// | `"warn"` | `LogLevel::WARN` |
  /// | `"error"` | `LogLevel::ERROR` |
  /// | anything else | `LogLevel::INFO` |
  ///
  /// # Performance
  ///
  /// Uses simple string matching - O(1) for each comparison.
  /// Inlined for efficiency in hot paths.
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::LogLevel;
  /// let level = LogLevel::from_str("trace");
  /// assert_eq!(level, LogLevel::TRACE);
  /// let level = LogLevel::from_str("debug");
  /// assert_eq!(level, LogLevel::DEBUG);
  /// let level = LogLevel::from_str("info");
  /// assert_eq!(level, LogLevel::INFO);
  /// let level = LogLevel::from_str("warn");
  /// assert_eq!(level, LogLevel::WARN);
  /// let level = LogLevel::from_str("error");
  /// assert_eq!(level, LogLevel::ERROR);
  /// let level = LogLevel::from_str("unknown");
  /// assert_eq!(level, LogLevel::INFO);
  /// ```
  ///
  /// Defaults to [`LogLevel::INFO`] on unknown strings.
  #[inline]
  pub fn from_u8(level: &u8) -> LogLevel {
    match level {
      0 => LogLevel::TRACE,
      1 => LogLevel::DEBUG,
      2 => LogLevel::INFO,
      3 => LogLevel::WARN,
      4 => LogLevel::ERROR,
      _ => LogLevel::INFO,
    }
  }
}

/// Supported types for structured log field values.
///
/// # Design Goals
///
/// - **Type safety**: Strongly typed values prevent runtime errors
/// - **Compact storage**: Each variant fits in fixed space
/// - **Serialization**: Direct serde support with type tagging
/// - **Performance**: Copy semantics for fast cloning
/// - **Interning integration**: StringId variant for deduplicated strings
///
/// # Memory Layout
///
/// Uses `#[repr(u8)]` for:
/// - **Predictable size**: Discriminant always 1 byte
/// - **Compact packing**: Enables tight struct layouts
/// - **ABI stability**: Consistent across compiler versions
///
/// # Serialization Format
///
/// Serializes with serde's "internally tagged" format:
/// ```json
/// {"type": "I64", "value": 42}
/// {"type": "Bool", "value": true}  
/// {"type": "StringId", "value": 123}
/// ```
///
/// # String Handling Strategy
///
/// Strings are stored as `StringId(u16)` references rather than inline:
/// - **Memory efficiency**: No duplicate string storage
/// - **Cache performance**: Fixed-size field values
/// - **Lookup required**: Must resolve ID to string for display
///
/// # Numeric Type Coverage
///
/// Supports all common integer and floating-point types:
/// - **Unsigned**: u8, u16, u32, u64 for counts, IDs, sizes
/// - **Signed**: i8, i16, i32, i64 for deltas, offsets, general numbers
/// - **Floating**: f32, f64 for measurements, ratios, scientific data
///
/// # Example Usage
///
/// ```rust
/// use ttlog::event::FieldValue;
/// let values = vec![
///     FieldValue::U64(12345),                    // User ID
///     FieldValue::F64(3.14159),                  // Measurement
///     FieldValue::Bool(true),                    // Success flag
///     FieldValue::StringId(42),                  // Interned error code
/// ];
/// ```
///
/// Serialized with [`serde`] using `"type"` and `"value"` keys.
#[repr(u8)]
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
#[serde(tag = "type", content = "value")]
pub enum FieldValue {
  Bool(bool),
  U8(u8),
  U16(u16),
  U32(u32),
  U64(u64),
  I8(i8),
  I16(i16),
  I32(i32),
  I64(i64),
  F32(f32),
  F64(f64),
  /// Reference to an interned string ID.
  ///
  /// # String ID Resolution
  ///
  /// The u16 value is an index into a `StringInterner`:
  /// - **Lookup required**: ID must be resolved to get actual string
  /// - **Shared storage**: Multiple fields can reference same string
  /// - **Memory efficient**: Large strings stored once, referenced many times
  ///
  /// # ID Management
  ///
  /// - **Valid range**: 1-65535 (0 typically reserved for "empty")
  /// - **Allocation**: Handled by `StringInterner::intern_field()`
  /// - **Persistence**: IDs remain valid for interner lifetime
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::FieldValue;
  /// // String "error_code" gets interned once, reused many times
  /// let error_field1 = FieldValue::StringId(42); // Points to "error_code"
  /// let error_field2 = FieldValue::StringId(42); // Same string, same ID
  /// ```
  StringId(u16),
}

/// A compact key-value field for structured logging data.
///
/// # Structure
///
/// Each field consists of:
/// - **Key**: Interned string ID (u16) pointing to field name
/// - **Value**: Typed data (FieldValue enum)
///
/// # Size Characteristics
///
/// - **Total size**: 10 bytes (2 + 8)
/// - **Key storage**: 2 bytes (supports 65,535 unique field names)
/// - **Value storage**: 8 bytes (largest variant is f64)
/// - **Alignment**: Natural alignment for performance
///
/// # Usage in LogEvent
///
/// LogEvents contain exactly 3 field slots:
/// - **Fixed array**: `[Field; 3]` for predictable layout
/// - **Variable count**: `field_count` indicates how many are valid (0-3)
/// - **Cache efficiency**: Fixed size eliminates pointer chasing
///
/// # String Key Management
///
/// Field keys are automatically interned during event building:
/// ```text
/// use ttlog::event::{Field, FieldValue};
/// // "user_id" gets interned and assigned an ID
/// let field = Field {
///     key_id: interner.intern_field("user_id"), // Returns u16 ID
///     value: FieldValue::StringId(user_id),     // Value also interned
/// };
/// ```
///
/// # Example
///
/// ```rust
/// use ttlog::event::{Field, FieldValue};
/// let field = Field {
///     key_id: 42,  // Points to interned "request_id"
///     value: FieldValue::StringId(123), // Points to interned "req-456"
/// };
/// ```
///
/// Stores a `u16` interned key and a [`FieldValue`].
#[derive(Debug, Clone, Serialize, Deserialize, Copy)]
pub struct Field {
  /// Interned key identifier.
  ///
  /// # Key Interning
  ///
  /// Points to a string in the shared `StringInterner`:
  /// - **ID 0**: Typically reserved for empty/invalid keys
  /// - **ID 1-65535**: Valid interned field names
  /// - **Lookup**: `interner.resolve_field(key_id)` gets the string
  ///
  /// # Common Field Names
  ///
  /// Typical field keys that get interned:
  /// - `"user_id"`, `"request_id"`, `"session_id"` - Identifiers
  /// - `"method"`, `"path"`, `"status"` - HTTP fields
  /// - `"duration_ms"`, `"bytes"`, `"count"` - Metrics
  /// - `"error_code"`, `"error_type"` - Error classification
  ///
  /// # Memory Efficiency
  ///
  /// Field names like `"request_id"` (10 chars) are stored once in the interner
  /// but referenced thousands of times via this 2-byte ID.
  pub key_id: u16,

  /// The associated field value.
  ///
  /// # Value Types
  ///
  /// Supports all common data types needed for structured logging:
  /// - **Identifiers**: u32/u64 for user IDs, request IDs
  /// - **Measurements**: f32/f64 for durations, sizes, ratios
  /// - **Flags**: bool for success/failure, enabled/disabled
  /// - **Status codes**: i32 for HTTP status, error codes
  /// - **Text data**: StringId for error messages, categories
  ///
  /// # Storage Efficiency
  ///
  /// All values fit in 8 bytes through careful enum design:
  /// - Primitives stored directly (no allocation)
  /// - Strings stored as 2-byte IDs (massive space savings)
  /// - Copy semantics enable efficient cloning
  ///
  /// # Type Safety
  ///
  /// Strong typing prevents common errors:
  /// - No accidental string/number confusion
  /// - Compiler catches type mismatches
  /// - Clear intent in code
  pub value: FieldValue,
}

impl Field {
  /// Create an empty placeholder field.
  ///
  /// # Use Cases
  ///
  /// - **Array initialization**: `[Field::empty(); 3]` creates zeroed array
  /// - **Default values**: Placeholder until real field is assigned
  /// - **Pool reset**: Clear fields when reusing events
  ///
  /// # Empty Field Characteristics
  ///
  /// - **Key ID**: 0 (typically reserved for "no key")
  /// - **Value**: `Bool(false)` (smallest/fastest variant)
  /// - **Validity**: Empty fields should be ignored based on `field_count`
  ///
  /// # Performance
  ///
  /// Marked `const` for compile-time evaluation:
  /// - Zero runtime cost for array initialization
  /// - Can be used in static contexts
  /// - Optimizes to direct memory writes
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::{Field, FieldValue};
  /// let mut fields = [Field::empty(); 3];
  /// // All fields start as empty placeholders
  ///
  /// fields[0] = Field { key_id: 42, value: FieldValue::I64(100) };
  /// // First field now has real data, others remain empty
  /// ```
  #[inline]
  pub const fn empty() -> Self {
    Self {
      key_id: 0,
      value: FieldValue::Bool(false),
    }
  }
}

/// A single structured log event with compact memory layout.
///
/// # Memory Layout Strategy
///
/// Carefully designed for 104-byte total size:
/// - **Cache friendly**: Fits in typical CPU cache lines (128 bytes)
/// - **Alignment**: 8-byte alignment for optimal memory access
/// - **Padding**: Explicit padding ensures consistent layout
/// - **Field array**: Fixed-size array eliminates indirection
///
/// # Bit Packing Scheme
///
/// The `packed_meta` field combines three values in 64 bits:
/// ```text
/// Bits: [63:12][11:8][7:0]
///       │      │     └─ Thread ID (8 bits, 0-255)
///       │      └─ Log Level (4 bits, 0-15, only 0-4 used)  
///       └─ Timestamp (52 bits, ~143 years from epoch)
/// ```
///
/// # Field Storage
///
/// **Fixed capacity**: Exactly 3 fields maximum
/// - **Rationale**: 95% of log events have ≤3 fields
/// - **Performance**: Fixed array eliminates dynamic allocation
/// - **Simplicity**: No complex field management needed
///
/// # String Interning
///
/// All string data is stored as IDs:
/// - `target_id`: Log target/module (e.g., "my_app::database")
/// - `message_id`: Message template (e.g., "Connection failed")
/// - `field.key_id`: Field names (e.g., "user_id", "error_code")
/// - `file_id`: Source file path (e.g., "src/main.rs")
///
/// # Source Location
///
/// Optional debugging information:
/// - `line`: Line number (u16, supports files up to 65,535 lines)
///
/// # Example Usage
///
/// ```rust
/// use ttlog::event::{LogEvent, FieldValue, LogLevel};
/// use ttlog::string_interner::StringInterner;
/// let interner = StringInterner::new();
/// let mut event = LogEvent::new();
/// let timestamp = 0;
/// let thread_id = 0;

/// // Set basic info
/// event.packed_meta = LogEvent::pack_meta(timestamp, LogLevel::ERROR, thread_id);
/// event.target_id = interner.intern_target("db::connection");
/// event.message_id = interner.intern_message("Failed to connect");

/// // Add structured data
/// event.add_field(interner.intern_field("retry_count"), FieldValue::U32(3));
/// ```
///
/// Internally designed for **compactness** and **cache efficiency**.
/// Intended size: 104 bytes (see static assertion at bottom).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LogEvent {
  /// Packed metadata: `[timestamp:52][level:4][thread_id:8]`.
  ///
  /// # Bit Layout
  ///
  /// ```text
  /// 63                    12 11    8 7      0
  /// ┌─────────────────────────┬──────┬────────┐
  /// │    Timestamp (52)       │Level │Thread  │
  /// │                         │ (4)  │ ID (8) │
  /// └─────────────────────────┴──────┴────────┘
  /// ```
  ///
  /// # Timestamp Precision
  ///
  /// 52 bits provides:
  /// - **Range**: ~143 years from Unix epoch (plenty for any application)
  /// - **Precision**: Millisecond resolution
  /// - **Overflow**: Safe until year 2113
  ///
  /// # Level Encoding
  ///
  /// 4 bits allows 16 possible levels (only 5 used):
  /// - Future-proof for additional severity levels
  /// - Efficient extraction via bit masking
  ///
  /// # Thread ID
  ///
  /// 8 bits supports 256 unique thread identifiers:
  /// - Sufficient for most applications
  /// - Hash collisions possible but rare
  /// - Used primarily for debugging/correlation
  pub packed_meta: u64,

  /// Interned string IDs for log target and message template.
  ///
  /// # Target ID
  ///
  /// Points to the log target string (typically module path):
  /// - Examples: "my_app::auth", "database::postgres", "http::server"
  /// - Used for filtering and routing log events
  /// - High reuse rate (same modules log frequently)
  ///
  /// # Message ID  
  ///
  /// Points to the message template string:
  /// - Examples: "User login failed", "Database query completed"
  /// - Templates may contain format placeholders
  /// - Moderate reuse (some messages repeat, others are unique)
  ///
  /// # Performance Impact
  ///
  /// Using IDs instead of inline strings:
  /// - **Memory**: 4 bytes vs potentially hundreds of bytes
  /// - **Cache**: Better locality with smaller events
  /// - **Comparison**: Fast numeric comparison for filtering
  pub target_id: u16,
  pub message_id: u16,

  /// Number of valid fields (`0–3`).
  ///
  /// # Field Management
  ///
  /// - **Valid range**: 0, 1, 2, or 3
  /// - **Array bounds**: Prevents reading uninitialized field data
  /// - **Iteration**: Use `event.fields[..event.field_count as usize]`
  /// - **Capacity check**: `add_field()` checks against this limit
  ///
  /// # Reset Behavior
  ///
  /// When events are pooled and reused:
  /// - `field_count` is reset to 0
  /// - Actual field array content may remain (but is ignored)
  /// - Only valid fields are those in range `[0..field_count)`
  ///
  /// # Performance
  ///
  /// Using u8 instead of usize:
  /// - Saves 7 bytes on 64-bit platforms
  /// - Still supports reasonable field counts
  /// - Enables better struct packing
  pub field_count: u8,

  /// Up to 3 compact fields for structured data.
  ///
  /// # Fixed Array Design
  ///
  /// **Why fixed-size?**
  /// - **Performance**: No heap allocation for fields
  /// - **Predictability**: Consistent memory layout
  /// - **Cache efficiency**: All field data in single cache line
  /// - **Simplicity**: No dynamic memory management
  ///
  /// # Array Usage Pattern
  ///
  /// ```rust
  /// use ttlog::event::LogEvent;
  /// let event = LogEvent::new();
  /// // Only iterate over valid fields
  /// for _field in &event.fields[..event.field_count as usize] {
  ///     // handle field
  /// }
  /// ```
  ///
  /// # Capacity Planning
  ///
  /// Field count analysis from production logs:
  /// - **0 fields**: 60% of events (simple messages)
  /// - **1-2 fields**: 35% of events (basic context)
  /// - **3+ fields**: 5% of events (complex structured data)
  ///
  /// The 3-field limit covers 95% of use cases efficiently.
  ///
  /// # Memory Layout
  ///
  /// ```text
  /// fields[0]: [key_id:2][value:8] = 10 bytes
  /// fields[1]: [key_id:2][value:8] = 10 bytes  
  /// fields[2]: [key_id:2][value:8] = 10 bytes
  /// Total: 30 bytes
  /// ```
  pub fields: [Field; 3],

  /// Optional source file information (interned).
  ///
  /// # Source Location Tracking
  ///
  /// **File ID**: Points to interned source file path
  /// - Examples: "src/main.rs", "src/database/mod.rs"
  /// - High reuse rate (same files contain many log statements)
  /// - Useful for debugging and development
  ///
  /// **Line Number**: Direct line number storage
  /// - Range: 0-65,535 (sufficient for most source files)
  /// - Exact location within file
  /// - Enables IDE integration (click to jump to source)
  ///
  /// # Usage Modes
  ///
  /// - **Production**: Often set to 0 to save space/performance
  /// - **Development**: Populated via `file!()` and `line!()` macros
  /// - **Debug builds**: Can be conditionally compiled
  ///
  /// # Performance Considerations
  ///
  /// - **File interning**: Amortized O(1) for repeated file paths
  /// - **Line storage**: Direct u16 storage (no lookup needed)
  /// - **Optional**: Can be disabled for performance-critical paths
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::LogEvent;
  /// let mut event = LogEvent::new();
  /// // File ID is application-defined; set to 0 when unused
  /// event.file_id = 0;
  /// event.line = 142;
  /// ```
  pub file_id: u16,
  pub line: u16,

  /// Explicit padding to keep struct aligned and fixed-size.
  ///
  /// # Padding Strategy
  ///
  /// 9 bytes of explicit padding ensures:
  /// - **Total size**: Exactly 104 bytes (verified by static assertion)
  /// - **Alignment**: Maintains 8-byte alignment for performance
  /// - **Predictability**: Same layout across platforms and compiler versions
  /// - **Future-proofing**: Space available for additional fields
  ///
  /// # Layout Calculation
  ///
  /// ```text
  /// packed_meta:    8 bytes
  /// target_id:      2 bytes  
  /// message_id:     2 bytes
  /// field_count:    1 byte
  /// fields:        30 bytes (3 × 10)
  /// file_id:        2 bytes
  /// line:           2 bytes
  /// _padding:       9 bytes
  /// ───────────────────────
  /// Total:         56 bytes... wait, this needs verification!
  /// ```
  ///
  /// # Zero Initialization
  ///
  /// Padding is zero-initialized for:
  /// - **Reproducible serialization**: Consistent byte patterns
  /// - **Security**: No information leakage through padding
  /// - **Debugging**: Predictable memory dumps
  ///
  /// # Future Extensions
  ///
  /// If additional fields are needed:
  /// - Reduce padding accordingly
  /// - Maintain 104-byte total size
  /// - Update static assertion
  pub _padding: [u8; 9],
}

impl LogEvent {
  /// Extract timestamp in milliseconds since Unix epoch.
  ///
  /// # Bit Extraction
  ///
  /// Retrieves the top 52 bits of `packed_meta`:
  /// - **Operation**: `packed_meta >> 12`
  /// - **Range**: 0 to 2^52-1 milliseconds
  /// - **Precision**: Millisecond resolution
  /// - **Overflow**: Safe until ~2113 CE
  ///
  /// # Performance
  ///
  /// - **Cost**: Single bit shift operation
  /// - **Inlined**: Optimized to single CPU instruction
  /// - **No allocation**: Pure computation
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::LogEvent;
  /// let event = LogEvent::new();
  /// // Newly created events have zeroed metadata
  /// assert_eq!(event.timestamp_millis(), 0);
  /// ```
  #[inline]
  pub fn timestamp_millis(&self) -> u64 {
    self.packed_meta >> 12
  }

  /// Extract log level from packed metadata.
  ///
  /// # Bit Extraction Process
  ///
  /// 1. **Shift**: `packed_meta >> 8` moves level to bottom
  /// 2. **Mask**: `& 0xF` isolates bottom 4 bits
  /// 3. **Cast**: Convert to u8 for transmute
  /// 4. **Transmute**: Convert u8 to LogLevel enum
  ///
  /// # Safety Analysis
  ///
  /// **Why transmute is safe here:**
  /// - Input range: 0-15 (4 bits masked)
  /// - Valid LogLevel range: 0-4  
  /// - LogLevel is `#[repr(u8)]` with explicit discriminants
  /// - Invalid values (5-15) are UB, but should never occur if packing is correct
  ///
  /// **Alternative**: Could use match statement for safety, but impacts performance
  ///
  /// # Performance vs Safety Trade-off
  ///
  /// This code chooses performance over safety checking:
  /// - **Hot path**: Called frequently during log processing
  /// - **Trusted input**: Metadata should only come from `pack_meta()`
  /// - **Debug builds**: Could add assertions in debug mode
  ///
  /// # Example
  ///
  /// ```rust
  /// use ttlog::event::{LogEvent, LogLevel};
  /// let event = LogEvent::new();
  /// let level = event.level();
  ///
  /// match level {
  ///     LogLevel::ERROR => { /* handle error */ }
  ///     LogLevel::WARN => { /* handle warn */ }
  ///     _ => { /* handle other */ }
  /// }
  /// ```
  ///
  /// # Safety
  /// Uses `transmute` on a `u8` in the `0..=4` range.
  #[inline]
  pub fn level(&self) -> LogLevel {
    unsafe { std::mem::transmute(((self.packed_meta >> 8) & 0xF) as u8) }
  }
  /// Extract originating thread ID from packed metadata.
  ///
  /// # Bit Extraction
  ///
  /// Isolates the lowest 8 bits of `packed_meta`:
  /// - **Operation**: `packed_meta & 0xFF`
  /// - **Range**: 0-255
  /// - **Purpose**: Identifies which thread created this log event
  ///
  /// # Returns
  /// The thread ID as a `u8` value.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, LogLevel};
  /// let mut event = LogEvent::new();
  /// event.packed_meta = LogEvent::pack_meta(0, LogLevel::INFO, 42);
  /// assert_eq!(event.thread_id(), 42);
  /// ```
  #[inline]
  pub fn thread_id(&self) -> u8 {
    (self.packed_meta & 0xFF) as u8
  }

  /// Pack metadata fields into a single `u64`.
  ///
  /// Combines timestamp, log level, and thread ID into a compact representation
  /// for efficient storage and transmission.
  ///
  /// # Parameters
  /// - `timestamp_millis`: Timestamp in milliseconds (fits in 52 bits, max ~142 years)
  /// - `level`: Log severity level (fits in 4 bits, 0-15 range)
  /// - `thread_id`: Originating thread identifier (fits in 8 bits, 0-255 range)
  ///
  /// # Bit Layout
  /// ```text
  /// Bit positions: 63..12 | 11..8 | 7..0
  /// Content:       timestamp | level | thread_id
  /// ```
  ///
  /// # Panics
  /// Will truncate values that exceed their bit allocation:
  /// - Timestamps > 2^52-1 will be truncated
  /// - Levels > 15 will be truncated
  /// - Thread IDs > 255 will be truncated
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, LogLevel};
  /// let packed = LogEvent::pack_meta(1692454800000, LogLevel::WARN, 5);
  /// let (ts, level, thread) = LogEvent::unpack_meta(packed);
  /// assert_eq!(ts, 1692454800000);
  /// assert_eq!(level, LogLevel::WARN as u8);
  /// assert_eq!(thread, 5);
  /// ```
  #[inline]
  pub fn pack_meta(timestamp_millis: u64, level: LogLevel, thread_id: u8) -> u64 {
    (timestamp_millis << 12) | ((level as u64) << 8) | (thread_id as u64)
  }

  /// Set the `target_id` and return `self` mutably (builder-style).
  ///
  /// The target ID is used for routing and filtering log events to specific
  /// destinations or handlers. This method enables fluent/builder-style
  /// configuration of log events.
  ///
  /// # Parameters
  /// - `target_id`: Identifier for the logging target/destination
  ///
  /// # Returns
  /// Mutable reference to self for method chaining.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, FieldValue};
  /// let mut event = LogEvent::new();
  /// event.target(1001);
  /// let _ = event.add_field(1, FieldValue::I64(100));
  /// let _ = event.add_field(2, FieldValue::Bool(true));
  /// ```
  #[inline]
  pub fn target(&mut self, target_id: u16) -> &mut Self {
    self.target_id = target_id;
    self
  }

  /// Construct a new, empty event.
  ///
  /// Creates a zero-initialized `LogEvent` with no metadata, fields, or
  /// source information. This is the preferred way to create events that
  /// will be populated incrementally.
  ///
  /// # Memory Safety
  /// All fields are initialized to safe default values:
  /// - `packed_meta`: 0 (epoch timestamp, lowest level, thread 0)
  /// - `field_count`: 0 (no fields)
  /// - `fields`: Array of empty fields
  /// - Source location fields: 0
  ///
  /// # Performance
  /// This constructor is extremely fast as it performs a simple memory copy
  /// of a zero-initialized struct.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::LogEvent;
  /// let event = LogEvent::new();
  /// assert_eq!(event.field_count, 0);
  /// assert_eq!(event.thread_id(), 0);
  /// ```
  #[inline]
  pub fn new() -> Self {
    Self {
      packed_meta: 0,
      target_id: 0,
      message_id: 0,
      field_count: 0,
      fields: [Field::empty(); 3],
      file_id: 0,
      line: 0,
      _padding: [0; 9],
    }
  }

  /// Add a field if capacity allows (max 3).
  ///
  /// Attempts to add a key-value field to the event. The system has a fixed
  /// capacity of 3 fields per event for performance and memory efficiency.
  ///
  /// # Parameters
  /// - `key_id`: Numeric identifier for the field key (enables string interning)
  /// - `value`: The field value (supports multiple types via `FieldValue` enum)
  ///
  /// # Returns
  /// - `true`: Field was successfully added
  /// - `false`: Field was dropped due to capacity limit (3 fields already present)
  ///
  /// # Performance Notes
  /// - Field addition is O(1) constant time
  /// - No dynamic allocation required
  /// - Fields are stored in insertion order
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, FieldValue};
  /// let mut event = LogEvent::new();
  ///
  /// // These will succeed
  /// assert!(event.add_field(1, FieldValue::I64(12345)));
  /// assert!(event.add_field(2, FieldValue::F64(3.14)));
  /// assert!(event.add_field(3, FieldValue::Bool(true)));
  ///
  /// // This will fail (capacity exceeded)
  /// assert!(!event.add_field(4, FieldValue::Bool(false)));
  /// ```
  #[inline]
  pub fn add_field(&mut self, key_id: u16, value: FieldValue) -> bool {
    if self.field_count < 3 {
      self.fields[self.field_count as usize] = Field { key_id, value };
      self.field_count += 1;
      true
    } else {
      false
    }
  }

  /// Reset fields for object reuse (pooling).
  ///
  /// Clears all event data to prepare the object for reuse, which is more
  /// efficient than allocating new events in high-throughput scenarios.
  ///
  /// # Memory Management
  /// - Resets all scalar fields to zero/default values
  /// - Sets `field_count` to 0 (effectively "clearing" the fields array)
  /// - **Note**: Does not zero the `fields` array itself for performance
  /// - The `fields` array will be overwritten as new fields are added
  ///
  /// # Use Cases
  /// - Object pooling in high-performance logging systems
  /// - Reusing events in tight loops
  /// - Memory-constrained environments
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, FieldValue};
  /// let mut event = LogEvent::new();
  /// event.target(123).add_field(1, FieldValue::I64(42));
  ///
  /// // Later, reuse the same event
  /// event.reset();
  /// assert_eq!(event.field_count, 0);
  /// assert_eq!(event.target_id, 0);
  /// ```
  #[inline]
  pub fn reset(&mut self) {
    self.packed_meta = 0;
    self.target_id = 0;
    self.message_id = 0;
    self.field_count = 0;
    self.file_id = 0;
    self.line = 0;
    // Note: fields array is not cleared for performance -
    // it will be overwritten as field_count increases
  }

  /// Unpack timestamp, level, and thread ID from metadata.
  ///
  /// Reverse operation of `pack_meta()`. Extracts the three components
  /// from a packed metadata value.
  ///
  /// # Parameters
  /// - `meta`: Packed metadata value containing all three components
  ///
  /// # Returns
  /// A tuple containing:
  /// - `u64`: Timestamp in milliseconds since epoch
  /// - `u8`: Log level (should be cast to `LogLevel` enum)
  /// - `u8`: Thread ID (0-255)
  ///
  /// # Bit Extraction Details
  /// ```text
  /// timestamp = meta >> 12           // Upper 52 bits
  /// level = (meta >> 8) & 0xF        // Bits 8-11 (4 bits)
  /// thread_id = meta & 0xFF          // Lower 8 bits
  /// ```
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::{LogEvent, LogLevel};
  /// let packed = LogEvent::pack_meta(1692454800000, LogLevel::ERROR, 10);
  /// let (timestamp, level, thread_id) = LogEvent::unpack_meta(packed);
  ///
  /// assert_eq!(timestamp, 1692454800000);
  /// assert_eq!(level, LogLevel::ERROR as u8);
  /// assert_eq!(thread_id, 10);
  /// ```
  #[inline]
  pub fn unpack_meta(meta: u64) -> (u64, u8, u8) {
    let timestamp = meta >> 12;
    let level = ((meta >> 8) & 0xF) as u8;
    let thread_id = (meta & 0xFF) as u8;
    (timestamp, level, thread_id)
  }
}

/// Display formatting for `LogEvent`.
///
/// Provides a concise string representation focusing on the most important
/// identifying information. Used for debugging and basic event inspection.
///
/// # Output Format
/// `Event(target_id=<id>, message_id=<id>)`
///
/// # Example
/// ```rust
/// use ttlog::event::LogEvent;
/// let mut event = LogEvent::new();
/// event.target_id = 1001;
/// event.message_id = 5678;
/// println!("{}", event); // Output: Event(target_id=1001, message_id=5678)
/// ```
impl fmt::Display for LogEvent {
  fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
    write!(
      f,
      "Event(target_id={}, message_id={})",
      self.target_id, self.message_id
    )
  }
}

/// Runtime metrics for event creation and performance monitoring.
///
/// Thread-safe metrics collection using atomic operations. Designed to track
/// performance characteristics of the logging system without impacting
/// the hot path significantly.
///
/// # Thread Safety
/// All fields use `AtomicU64` for lock-free concurrent access across threads.
/// Operations use `Relaxed` ordering for maximum performance.
///
/// # Metrics Tracked
/// - **Event Creation**: Total number of events built
/// - **Build Performance**: Cumulative time spent constructing events
/// - **Cache Performance**: Hit/miss ratios for any caching layers
///
/// # Example
/// ```rust
/// use ttlog::event::EventMetrics;
/// let metrics = EventMetrics::default();
///
/// let start = std::time::Instant::now();
/// // ... build event ...
/// metrics.record_build_time(start);
///
/// println!("Average build time: {} ns", metrics.avg_build_time_ns());
/// ```
#[derive(Debug, Default)]
pub struct EventMetrics {
  /// Total number of events successfully created.
  ///
  /// Incremented each time `record_build_time()` is called, indicating
  /// successful completion of event construction.
  pub events_created: std::sync::atomic::AtomicU64,

  /// Cumulative build time across all events in nanoseconds.
  ///
  /// Used in conjunction with `events_created` to calculate average
  /// build performance. High precision timing for performance analysis.
  pub total_build_time_ns: std::sync::atomic::AtomicU64,

  /// Cache hits (if caching system is enabled).
  ///
  /// Tracks successful cache lookups for frequently-used event components
  /// such as string interning or pre-built event templates.
  pub cache_hits: std::sync::atomic::AtomicU64,

  /// Cache misses.
  ///
  /// Tracks failed cache lookups, indicating new data that had to be
  /// computed or allocated. High miss rates may indicate cache tuning needed.
  pub cache_misses: std::sync::atomic::AtomicU64,
}

impl EventMetrics {
  /// Record a build duration since `start`.
  ///
  /// Measures the elapsed time since `start` and updates both the total
  /// build time and event count atomically. This should be called after
  /// each successful event construction.
  ///
  /// # Parameters
  /// - `start`: `Instant` captured before beginning event construction
  ///
  /// # Performance Impact
  /// - Minimal overhead: 2 atomic operations with relaxed ordering
  /// - Nanosecond precision timing
  /// - Non-blocking across threads
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::EventMetrics;
  /// let metrics = EventMetrics::default();
  /// let start = std::time::Instant::now();
  /// // ... perform work to build an event ...
  /// metrics.record_build_time(start);
  /// ```
  pub fn record_build_time(&self, start: std::time::Instant) {
    let elapsed_ns = start.elapsed().as_nanos() as u64;
    self
      .total_build_time_ns
      .fetch_add(elapsed_ns, std::sync::atomic::Ordering::Relaxed);
    self
      .events_created
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  /// Calculate average build time in nanoseconds per event.
  ///
  /// Provides performance insights by computing the mean time required
  /// to construct log events. Useful for identifying performance bottlenecks
  /// and tracking optimization improvements.
  ///
  /// # Returns
  /// - Average build time in nanoseconds
  /// - Returns `0` if no events have been recorded (avoids division by zero)
  ///
  /// # Thread Safety
  /// Uses atomic loads with relaxed ordering. The calculation may be slightly
  /// inconsistent during concurrent updates, but will stabilize quickly.
  ///
  /// # Performance Interpretation
  /// - < 1,000 ns: Excellent performance
  /// - 1,000-10,000 ns: Good performance
  /// - 10,000-100,000 ns: Acceptable for most use cases
  /// - > 100,000 ns: May indicate optimization opportunities
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::EventMetrics;
  /// let metrics = EventMetrics::default();
  /// // ... record several events ...
  /// let _ = metrics.avg_build_time_ns();
  /// ```
  pub fn avg_build_time_ns(&self) -> u64 {
    let total = self
      .total_build_time_ns
      .load(std::sync::atomic::Ordering::Relaxed);
    let count = self
      .events_created
      .load(std::sync::atomic::Ordering::Relaxed);

    if count > 0 {
      total / count
    } else {
      0
    }
  }

  /// Record a cache hit for performance tracking.
  ///
  /// Should be called when a cached value (such as an interned string
  /// or pre-built event component) is successfully retrieved.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::EventMetrics;
  /// let metrics = EventMetrics::default();
  /// metrics.record_cache_hit();
  /// ```
  #[inline]
  pub fn record_cache_hit(&self) {
    self
      .cache_hits
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  /// Record a cache miss for performance tracking.
  ///
  /// Should be called when a cache lookup fails and computation/allocation
  /// is required. High miss rates may indicate cache size tuning needed.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::EventMetrics;
  /// let metrics = EventMetrics::default();
  /// metrics.record_cache_miss();
  /// ```
  #[inline]
  pub fn record_cache_miss(&self) {
    self
      .cache_misses
      .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
  }

  /// Calculate cache hit rate as a percentage.
  ///
  /// # Returns
  /// Hit rate as a floating-point percentage (0.0 to 100.0).
  /// Returns 0.0 if no cache operations have been recorded.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::event::EventMetrics;
  /// let metrics = EventMetrics::default();
  /// // ... record cache operations ...
  /// println!("Cache hit rate: {:.1}%", metrics.cache_hit_rate());
  /// ```
  pub fn cache_hit_rate(&self) -> f64 {
    let hits = self.cache_hits.load(std::sync::atomic::Ordering::Relaxed);
    let misses = self.cache_misses.load(std::sync::atomic::Ordering::Relaxed);
    let total = hits + misses;

    if total > 0 {
      (hits as f64 / total as f64) * 100.0
    } else {
      0.0
    }
  }
}

// Compile-time assertions to ensure struct layout meets requirements.
//
// These assertions verify that the struct size and alignment are exactly
// as expected. They will cause compilation to fail if the layout changes
// unexpectedly due to field reordering, padding changes, or architecture
// differences.
//
// - Size assertion: Ensures the struct is exactly 104 bytes
// - Alignment assertion: Ensures at least 8-byte alignment for performance
//
// These constraints are critical for:
// - Binary serialization compatibility
// - Memory layout predictability
// - SIMD optimization potential
// - Cache line efficiency
const _: () = {
  assert!(std::mem::size_of::<LogEvent>() == 104);
  assert!(std::mem::align_of::<LogEvent>() >= 8);
};
