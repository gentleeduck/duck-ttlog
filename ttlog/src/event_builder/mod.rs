//! # EventBuilder Module
//!
//! High-performance event construction for logging systems with string interning,
//! object pooling, and minimal allocations.
//!
//! ## Features
//!
//! - **String Interning**: Automatically deduplicates targets, messages, and field keys
//! - **Object Pooling**: Maintains a circular pool of 16 reusable `LogEvent` objects per thread
//! - **Thread Safety**: Thread-local builders eliminate contention
//! - **Multiple Construction Modes**: Fast building, field building, and tracing integration
//! - **Zero-Copy**: Minimal allocations during event construction
//!
//! ## Usage Examples
//!
//! ```rust,ignore
//! use std::sync::Arc;
//! use crate::{EventBuilder, StringInterner, LogLevel, FieldValue};
//!
//! // Create shared interner
//! let interner = Arc::new(StringInterner::new());
//! let mut builder = EventBuilder::new(interner);
//!
//! // Fast event building
//! let event = builder.build_fast(
//!     1234567890,
//!     LogLevel::Info,
//!     "my_app::module",
//!     "Something happened"
//! );
//!
//! // Event with structured fields
//! let fields = vec![
//!     ("user_id".to_string(), FieldValue::String("user123".to_string())),
//!     ("attempt".to_string(), FieldValue::Integer(3)),
//!     ("success".to_string(), FieldValue::Boolean(true)),
//! ];
//!
//! let event_with_fields = builder.build_with_fields(
//!     1234567890,
//!     LogLevel::Debug,
//!     "auth::login",
//!     "User login attempt",
//!     &fields,
//! );
//! ```
//!
//! ## Performance Characteristics
//!
//! - **Pool Management**: O(1) event retrieval from circular buffer
//! - **String Interning**: O(1) amortized string deduplication  
//! - **Memory Efficiency**: Bounded memory usage with configurable pool size
//! - **Thread Locality**: No cross-thread synchronization during normal operation

mod __test__;
use std::{sync::Arc, thread};

use crate::{
  event::{FieldValue, LogEvent, LogLevel},
  string_interner::StringInterner,
};

/// Builder for [`LogEvent`]s with string interning and object pooling.
///
/// # Design Goals
///
/// The `EventBuilder` is designed for high-throughput logging scenarios where:
/// - Event creation must be extremely fast (sub-microsecond)
/// - Memory allocations should be minimized
/// - String deduplication reduces memory footprint
/// - Thread-local access eliminates contention
///
/// # Thread Safety
///
/// Each thread maintains its own `EventBuilder` instance via thread-local storage.
/// The shared `StringInterner` handles concurrent access internally.
///
/// # Object Pool Behavior
///
/// Events are pooled in a circular buffer with 16 slots. When the pool is full,
/// older events are reused after being reset. This provides bounded memory usage
/// while maintaining performance.
///
/// # String Interning
///
/// All string data (targets, messages, field keys) are automatically interned:
/// - First occurrence: String is stored and assigned an ID
/// - Subsequent occurrences: Only the ID is stored, saving memory
/// - Thread-safe: Multiple threads can intern strings concurrently
///
/// # Example
///
/// ```rust,ignore
/// let interner = Arc::new(StringInterner::new());
/// let mut builder = EventBuilder::new(interner);
///
/// // This will intern "my_app::auth" and "User logged in"
/// let event1 = builder.build_fast(
///     1000,
///     LogLevel::Info,
///     "my_app::auth",
///     "User logged in"
/// );
///
/// // This reuses the interned strings, no new allocations
/// let event2 = builder.build_fast(
///     2000,
///     LogLevel::Info,
///     "my_app::auth",  // Same target - reuses ID
///     "User logged in" // Same message - reuses ID
/// );
/// ```
#[derive(Debug)]
pub struct EventBuilder {
  /// Shared string interner for targets, messages, and field keys.
  ///
  /// Wrapped in `Arc` for sharing across threads. The interner handles
  /// concurrent access internally with appropriate synchronization.
  interner: Arc<StringInterner>,

  /// Thread identifier (stable within process).
  ///
  /// Computed once during builder creation by hashing the current thread ID.
  /// This provides a stable, compact identifier that fits in a u8 for
  /// efficient packing with other metadata.
  thread_id: u8,

  /// Pool of reusable events (capacity 16).
  ///
  /// Circular buffer of pre-allocated `LogEvent` objects. When an event
  /// is requested via `get_pooled_event()`, it's reset and returned.
  /// This eliminates allocation overhead for high-frequency logging.
  ///
  /// # Pool Management
  /// - Maximum size: 16 events
  /// - Access pattern: Circular (wraps around after 16)
  /// - Reset policy: Events are reset before reuse
  /// - Growth: Lazy - only allocates when needed
  event_pool: Vec<LogEvent>,

  /// Current position in circular pool.
  ///
  /// Points to the next slot to use. Wraps around to 0 after reaching 15.
  /// This provides O(1) pool access without complex allocation tracking.
  pool_index: usize,
}

impl EventBuilder {
  /// Create a new builder with a given [`StringInterner`].
  ///
  /// # Arguments
  /// * `interner` - Shared string interner for deduplication
  ///
  /// # Thread ID Generation
  ///
  /// The thread ID is computed by hashing `std::thread::current().id()` to create
  /// a stable, compact identifier. While hash collisions are possible, they're
  /// rare enough that thread identification remains useful for debugging.
  ///
  /// # Pool Initialization
  ///
  /// The event pool starts empty and grows lazily up to 16 events as needed.
  /// This avoids upfront allocation costs for threads that don't log frequently.
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// let interner = Arc::new(StringInterner::new());
  /// let builder = EventBuilder::new(interner);
  /// // Builder is ready to create events efficiently
  /// ```
  pub fn new(interner: Arc<StringInterner>) -> Self {
    let thread_id = EventBuilder::current_thread_id_u64() as u8;

    Self {
      interner,
      thread_id,
      event_pool: Vec::with_capacity(16),
      pool_index: 0,
    }
  }

  /// Get a pooled event, reusing pre-allocated storage.
  ///
  /// # Behavior
  /// - Returns a mutable reference to a pooled `LogEvent`
  /// - If pool is empty or exhausted, allocates a new event
  /// - Always resets the event before returning it
  /// - Uses circular indexing (wraps after 16 events)
  ///
  /// # Memory Management
  ///
  /// The pool uses a circular buffer strategy:
  /// 1. Check if current pool position has an event
  /// 2. If not, push a new event to the pool
  /// 3. Reset the event to clear previous data
  /// 4. Advance pool index (with wraparound)
  ///
  /// This ensures bounded memory usage while providing fast access.
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// let mut builder = EventBuilder::new(interner);
  ///
  /// let event = builder.get_pooled_event();
  /// // Event is ready to use, previously reset
  /// event.field_count = 2;
  ///
  /// let another_event = builder.get_pooled_event();
  /// // This is a different event, also reset
  /// assert_eq!(another_event.field_count, 0);
  /// ```
  ///
  /// # Performance
  ///
  /// - **Best case**: O(1) - reuses existing pooled event
  /// - **Allocation case**: O(1) - single vector push when growing pool
  /// - **Reset cost**: O(1) - constant time event clearing
  ///
  /// Will reset the event before returning it.
  pub fn get_pooled_event(&mut self) -> &mut LogEvent {
    if self.event_pool.len() <= self.pool_index {
      self.event_pool.push(LogEvent::new());
    }

    let event = &mut self.event_pool[self.pool_index];
    self.pool_index = (self.pool_index + 1) % 16;

    event.reset();
    event
  }

  /// Build an event quickly without fields.
  ///
  /// # Arguments
  /// * `timestamp_millis` - Unix timestamp in milliseconds
  /// * `level` - Log level (Error, Warn, Info, Debug, Trace)
  /// * `target` - Log target (typically module path)
  /// * `message` - Log message content
  ///
  /// # Performance Characteristics
  ///
  /// This method is optimized for speed:
  /// - **No pooling overhead**: Creates fresh `LogEvent`
  /// - **String interning**: O(1) amortized for repeated strings
  /// - **Metadata packing**: Combines timestamp, level, and thread ID efficiently
  /// - **Inline hint**: Compiler optimization for hot paths
  ///
  /// # String Interning
  ///
  /// Both `target` and `message` are automatically interned:
  /// - New strings trigger allocation and storage
  /// - Repeated strings reuse existing IDs
  /// - No manual string management required
  ///
  /// # Metadata Packing
  ///
  /// The timestamp, log level, and thread ID are packed into a single u64:
  /// - Reduces memory footprint
  /// - Improves cache locality
  /// - Enables atomic operations (if needed)
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// let event = builder.build_fast(
  ///     chrono::Utc::now().timestamp_millis() as u64,
  ///     LogLevel::Info,
  ///     "my_app::database",
  ///     "Connection established"
  /// );
  ///
  /// assert_eq!(event.field_count, 0); // No fields
  /// assert!(event.target_id > 0);     // String was interned
  /// ```
  ///
  /// Does not use the pool (returns a fresh `LogEvent`).
  #[inline]
  pub fn build_fast(
    &mut self,
    timestamp_millis: u64,
    level: LogLevel,
    target: &str,
    message: &str,
  ) -> LogEvent {
    let mut event = LogEvent::new();

    // Pack metadata (timestamp + level + thread_id).
    event.packed_meta = LogEvent::pack_meta(timestamp_millis, level, self.thread_id);

    // Intern string identifiers (allocates only for new strings).
    event.target_id = self.interner.intern_target(target);
    event.message_id = self.interner.intern_message(message);

    event
  }

  /// Build an event with up to 3 structured fields.
  ///
  /// # Arguments
  /// * `timestamp_millis` - Unix timestamp in milliseconds
  /// * `level` - Log level
  /// * `target` - Log target (module path)
  /// * `message` - Log message content
  /// * `fields` - Vector of key-value pairs for structured data
  ///
  /// # Field Limitations
  ///
  /// **Important**: Only the first 3 fields are processed. Additional fields
  /// are silently ignored. This limitation exists for performance reasons:
  /// - Fixed-size field array eliminates dynamic allocation
  /// - Predictable memory layout improves cache performance
  /// - Most logging scenarios use 0-3 fields
  ///
  /// # Field Processing
  ///
  /// For each field:
  /// 1. Key string is interned for deduplication
  /// 2. Value is stored directly (no copying for primitives)
  /// 3. Field is added to the event's fixed-size array
  ///
  /// # Supported Field Types
  ///
  /// - `FieldValue::String(String)` - Text data
  /// - `FieldValue::Integer(i64)` - Numeric data  
  /// - `FieldValue::Boolean(bool)` - Boolean flags
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// let fields = vec![
  ///     ("user_id".to_string(), FieldValue::String("user123".to_string())),
  ///     ("attempt_count".to_string(), FieldValue::Integer(3)),
  ///     ("success".to_string(), FieldValue::Boolean(true)),
  ///     ("ignored_field".to_string(), FieldValue::String("won't be added".to_string())),
  /// ];
  ///
  /// let event = builder.build_with_fields(
  ///     timestamp,
  ///     LogLevel::Info,
  ///     "auth::service",
  ///     "Login attempt completed",
  ///     &fields,
  /// );
  ///
  /// assert_eq!(event.field_count, 3); // Only first 3 fields added
  /// ```
  ///
  /// # Performance Notes
  ///
  /// - **Base cost**: Same as `build_fast()`
  /// - **Field cost**: O(n) where n = min(fields.len(), 3)
  /// - **String interning**: O(1) amortized per unique field key
  /// - **Value storage**: Zero-copy for primitive types
  ///
  /// # Parameters
  /// - `fields`: vector of `(String, FieldValue)` pairs
  ///
  /// Extra fields beyond 3 are silently ignored.
  pub fn build_with_fields(
    &mut self,
    timestamp_millis: u64,
    level: LogLevel,
    target: &str,
    message: &str,
    fields: &[(String, FieldValue)],
  ) -> LogEvent {
    let mut event = self.build_fast(timestamp_millis, level, target, message);

    for (key, value) in fields.iter().take(3) {
      let key_id = self.interner.intern_field(key);
      event.add_field(key_id, *value);
    }

    event
  }

  /// Build an event directly from a [`tracing::Event`].
  ///
  /// # Integration with Tracing
  ///
  /// This method provides seamless integration with the `tracing` crate:
  /// - Extracts metadata (level, target) from tracing event
  /// - Uses current timestamp (not tracing event timestamp)
  /// - Employs `MessageVisitor` to extract the message field efficiently
  /// - Maintains zero-copy semantics where possible
  ///
  /// # Message Extraction
  ///
  /// The message is extracted using a custom `MessageVisitor` that:
  /// - Looks specifically for fields named "message"
  /// - Handles both string and debug-formatted values
  /// - Falls back to empty string if no message field exists
  /// - Avoids unnecessary allocations
  ///
  /// # Timestamp Behavior
  ///
  /// **Note**: This method uses `chrono::Utc::now()` rather than any timestamp
  /// from the tracing event. This ensures consistent timing across the logging
  /// system and avoids potential clock skew issues.
  ///
  /// # Limitations
  ///
  /// - **Fields**: Currently only extracts the message field, not arbitrary fields
  /// - **Spans**: Does not capture span context (could be added in future)
  /// - **Metadata**: Extracts level and target but not file/line info
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// use tracing::{info, Level};
  ///
  /// let interner = Arc::new(StringInterner::new());
  /// let mut builder = EventBuilder::new(interner);
  ///
  /// // This would typically be called from a tracing subscriber
  /// info!(target: "my_app::service", "Processing request");
  ///
  /// // In subscriber implementation:
  /// let event = builder.build_from_tracing(&tracing_event);
  /// ```
  ///
  /// # Performance
  ///
  /// - **Visitor overhead**: Minimal - only processes "message" field
  /// - **Timestamp cost**: One system call to get current time
  /// - **String interning**: Same as other build methods
  ///
  /// Uses a [`MessageVisitor`] to extract the log message efficiently.
  #[inline]
  pub fn build_from_tracing(&mut self, tracing_event: &tracing::Event) -> LogEvent {
    let timestamp_millis = chrono::Utc::now().timestamp_millis() as u64;
    let level = LogLevel::from_tracing_level(tracing_event.metadata().level());
    let target = tracing_event.metadata().target();

    let mut visitor = MessageVisitor::default();
    tracing_event.record(&mut visitor);
    let message = visitor.message.as_deref().unwrap_or("");

    self.build_fast(timestamp_millis, level, target, message)
  }

  /// Compute a stable numeric thread identifier.
  ///
  /// # Algorithm
  ///
  /// Uses `DefaultHasher` to hash the opaque `ThreadId` from `std::thread::current().id()`:
  /// 1. Get current thread's ID (opaque type)
  /// 2. Hash it using the default hasher
  /// 3. Cast to u32, then truncate to u8
  ///
  /// # Stability Guarantees
  ///
  /// - **Per-thread**: Same thread always produces same ID
  /// - **Per-process**: IDs are stable within a single process run
  /// - **Cross-process**: No guarantees - IDs may differ between runs
  /// - **Cross-platform**: Implementation may vary by platform
  ///
  /// # Collision Handling
  ///
  /// Since we truncate to u8 (256 possible values), collisions are possible
  /// with many threads. However:
  /// - Most applications have < 256 threads
  /// - Thread IDs are primarily for debugging, not critical functionality
  /// - Collisions don't affect correctness, only debugging clarity
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// let id1 = EventBuilder::current_thread_id_u64();
  /// let id2 = EventBuilder::current_thread_id_u64();
  /// assert_eq!(id1, id2); // Same thread, same ID
  /// ```
  ///
  /// Uses a hash of `std::thread::current().id()`.
  fn current_thread_id_u64() -> u32 {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    thread::current().id().hash(&mut hasher);
    hasher.finish() as u32
  }
}

/// Visitor for extracting the `"message"` field from a [`tracing::Event`].
///
/// # Purpose
///
/// The `tracing` crate uses a visitor pattern to access event fields. This visitor
/// specifically targets the "message" field, which contains the primary log content.
/// It's designed for efficiency and minimal allocation.
///
/// # Field Handling Strategy
///
/// 1. **Primary**: `record_str()` - handles string values directly
/// 2. **Fallback**: `record_debug()` - formats other types using `Debug`
/// 3. **Filtering**: Only processes fields named "message"
/// 4. **Priority**: String values take precedence over debug formatting
///
/// # Allocation Behavior
///
/// - **String fields**: Direct clone (one allocation)
/// - **Debug fields**: `format!` allocation only when necessary
/// - **Non-message fields**: No allocation (ignored)
/// - **Empty events**: No allocation (message remains `None`)
///
/// # Example Usage
///
/// ```rust,ignore
/// use tracing::field::Visit;
///
/// let mut visitor = MessageVisitor::default();
/// let field = tracing::field::Field::new("message", tracing::field::Kind::STR);
///
/// visitor.record_str(&field, "Hello, world!");
/// assert_eq!(visitor.message, Some("Hello, world!".to_string()));
/// ```
///
/// Avoids allocations unless absolutely required.
#[derive(Default)]
pub struct MessageVisitor {
  /// Extracted message, if present.
  ///
  /// - `None`: No "message" field found in the event
  /// - `Some(String)`: Message content (either direct string or debug-formatted)
  ///
  /// The visitor prioritizes string values over debug formatting. If both
  /// `record_str` and `record_debug` are called for the "message" field,
  /// the string value is preserved.
  pub message: Option<String>,
}

impl tracing::field::Visit for MessageVisitor {
  /// Record a string field value.
  ///
  /// # Filtering
  ///
  /// Only processes fields named "message". Other fields are ignored to
  /// minimize overhead and avoid unnecessary allocations.
  ///
  /// # String Handling
  ///
  /// Directly converts `&str` to `String` via `to_string()`. This is the
  /// most efficient path for string message fields.
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// // In tracing macro expansion:
  /// info!("User {} logged in", user_id);
  ///
  /// // Results in:
  /// visitor.record_str(&message_field, "User alice logged in");
  /// ```
  #[inline]
  fn record_str(&mut self, field: &tracing::field::Field, value: &str) {
    if field.name() == "message" {
      self.message = Some(value.to_string());
    }
  }

  /// Record a debug-formattable field value.
  ///
  /// # Fallback Behavior
  ///
  /// This method serves as a fallback for non-string message fields:
  /// - Numeric types: `42` → `"42"`
  /// - Complex types: Uses their `Debug` implementation
  /// - Custom types: Depends on their `Debug` formatting
  ///
  /// # Conditional Processing
  ///
  /// Only processes "message" fields that haven't already been set by `record_str`.
  /// This ensures string values take precedence over debug formatting.
  ///
  /// # Allocation Cost
  ///
  /// Uses `format!("{:?}", value)` which allocates a new `String`. This is
  /// acceptable since debug formatting is typically a fallback case.
  ///
  /// # Example
  ///
  /// ```rust,ignore
  /// // In tracing macro expansion:
  /// info!(count = %42);
  ///
  /// // Results in:
  /// visitor.record_debug(&count_field, &42);
  /// // message becomes Some("42".to_string())
  /// ```
  #[inline]
  fn record_debug(&mut self, field: &tracing::field::Field, value: &dyn std::fmt::Debug) {
    if field.name() == "message" && self.message.is_none() {
      self.message = Some(format!("{:?}", value));
    }
  }
}

/// Thread-local `EventBuilder` for maximum performance.
///
/// # Thread-Local Storage Strategy
///
/// Each thread maintains its own `EventBuilder` instance to:
/// - **Eliminate contention**: No cross-thread synchronization during building
/// - **Preserve pools**: Each thread has its own event pool
/// - **Reduce overhead**: No mutex/atomic operations in hot path
/// - **Improve locality**: Thread-local data stays in CPU cache
///
/// # Lazy Initialization
///
/// Builders are created on first use per thread:
/// - `RefCell<Option<EventBuilder>>` allows mutable access
/// - First call to `build_event_fast()` creates the builder
/// - Subsequent calls reuse the existing builder and its pool
///
/// # Memory Management
///
/// - **Per-thread overhead**: One builder + up to 16 pooled events
/// - **Total overhead**: Scales with active thread count
/// - **Cleanup**: Thread-local data is cleaned up when thread exits
///
/// Ensures each thread reuses its own builder and pool.
thread_local! {
    static BUILDER: std::cell::RefCell<Option<EventBuilder>> = std::cell::RefCell::new(None);
}

/// Global function to build events quickly from a [`tracing::Event`].
///
/// # Global API Design
///
/// This function provides a convenient, thread-safe API for event building:
/// - **No explicit builder management**: Handles thread-local builder internally
/// - **Automatic initialization**: Creates builder on first use per thread
/// - **Pool reuse**: Benefits from thread-local event pooling
/// - **String sharing**: All threads share the same `StringInterner`
///
/// # Performance Benefits
///
/// - **Thread-local access**: No synchronization overhead
/// - **Builder reuse**: Amortizes initialization cost across many events
/// - **Pool efficiency**: Reuses pre-allocated events within each thread
/// - **String interning**: Shared across all threads for maximum deduplication
///
/// # Usage in Subscribers
///
/// This function is designed to be called from tracing subscribers:
///
/// ```rust,ignore
/// impl<S> tracing_subscriber::layer::Layer<S> for MyLayer
/// where S: tracing::Subscriber {
///     fn on_event(&self, event: &tracing::Event, _ctx: Context<S>) {
///         let log_event = build_event_fast(self.interner.clone(), event);
///         // Process the log_event...
///     }
/// }
/// ```
///
/// # Thread Safety
///
/// - **StringInterner**: Thread-safe (uses internal synchronization)
/// - **EventBuilder**: Thread-local (no sharing between threads)
/// - **LogEvent**: Owned value (safe to move between threads)
///
/// # Error Handling
///
/// - **Initialization failure**: Should never fail in practice
/// - **Interner sharing**: `Arc::clone()` is infallible
/// - **Event building**: Inherits error handling from `build_from_tracing()`
///
/// Uses a thread-local [`EventBuilder`] internally.
pub fn build_event_fast(interner: Arc<StringInterner>, tracing_event: &tracing::Event) -> LogEvent {
  BUILDER.with(|builder_cell| {
    let mut builder_opt = builder_cell.borrow_mut();

    if builder_opt.is_none() {
      *builder_opt = Some(EventBuilder::new(interner));
    }

    builder_opt
      .as_mut()
      .unwrap()
      .build_from_tracing(tracing_event)
  })
}

/// Build an event on the stack without pooling.
///
/// # Use Cases
///
/// This function is useful when:
/// - **No builder available**: Static contexts or initialization code
/// - **One-off events**: Don't benefit from pooling overhead
/// - **Testing**: Predictable behavior without pool state
/// - **Memory constraints**: Avoid thread-local storage overhead
///
/// # vs. Other Build Methods
///
/// | Method | Pooling | Thread-local | Tracing Integration |
/// |--------|---------|--------------|-------------------|
/// | `build_fast()` | No | No | No |
/// | `build_event_fast()` | Yes | Yes | Yes |
/// | `build_event_stack()` | No | No | No |
///
/// # Performance Trade-offs
///
/// **Advantages**:
/// - No thread-local lookup overhead
/// - No pool management complexity
/// - Predictable allocation behavior
/// - Simple call stack
///
/// **Disadvantages**:
/// - Allocates new `LogEvent` every time
/// - No amortization of builder creation
/// - Duplicates thread ID computation
///
/// # Thread ID Computation
///
/// Each call recomputes the thread ID using the same algorithm as `EventBuilder`.
/// For high-frequency logging, prefer `build_event_fast()` to amortize this cost.
///
/// # Example
///
/// ```rust,ignore
/// // Suitable for initialization or low-frequency logging
/// let event = build_event_stack(
///     &interner,
///     startup_time,
///     LogLevel::Info,
///     "app::startup",
///     "Application initialized"
/// );
/// ```
///
/// # Memory Layout
///
/// Creates a `LogEvent` with:
/// - Packed metadata (timestamp + level + thread_id)
/// - Interned string IDs for target and message
/// - Zero fields (field_count = 0)
/// - Default values for file/line information
/// - Zero-initialized padding
///
/// Unlike [`build_event_fast`], this does not use a thread-local builder.
#[inline]
pub fn build_event_stack(
  interner: &Arc<StringInterner>,
  timestamp_millis: u64,
  level: LogLevel,
  target: &str,
  message: &str,
) -> LogEvent {
  let thread_id = EventBuilder::current_thread_id_u64() as u8;

  LogEvent {
    packed_meta: LogEvent::pack_meta(timestamp_millis, level, thread_id),
    target_id: interner.intern_target(target),
    message_id: interner.intern_message(message),
    field_count: 0,
    fields: [crate::event::Field::empty(); 3],
    file_id: 0,
    line: 0,
    _padding: [0; 9],
  }
}
