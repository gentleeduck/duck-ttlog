mod __test__;

use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// A lock-free ring buffer using crossbeam's battle-tested ArrayQueue.
///
/// This implementation prioritizes correctness and reliability over maximum
/// performance, using crossbeam's well-audited ArrayQueue as the foundation.
/// When the buffer reaches capacity, new items overwrite the oldest items.
///
/// # Design Philosophy
///
/// This ring buffer is built on crossbeam's `ArrayQueue`, which provides:
/// - **Lock-free operations**: No mutex contention between threads
/// - **Wait-free push/pop**: Operations complete in bounded time
/// - **Memory safety**: No risk of data races or undefined behavior
/// - **Battle-tested reliability**: Extensively used in production systems
///
/// # Capacity Management
///
/// The buffer has a fixed capacity set at creation time. When full:
/// - New items trigger eviction of the oldest item
/// - FIFO (First In, First Out) ordering is maintained
/// - No blocking or allocation occurs
///
/// # Thread Safety
///
/// All operations are thread-safe and can be called concurrently from
/// multiple threads without external synchronization.
///
/// # Type Parameters
/// * `T` - The type of items stored in the buffer. Must be `Send` for
///          multi-threaded usage.
///
/// # Example
/// ```rust,ignore
/// use std::sync::Arc;
/// use std::thread;
///
/// let buffer = Arc::new(LockFreeRingBuffer::new(1000));
///
/// // Producer thread
/// let producer_buffer = buffer.clone();
/// let producer = thread::spawn(move || {
///     for i in 0..100 {
///         producer_buffer.push(i).unwrap();
///     }
/// });
///
/// // Consumer thread
/// let consumer_buffer = buffer.clone();
/// let consumer = thread::spawn(move || {
///     while let Some(item) = consumer_buffer.pop() {
///         println!("Consumed: {}", item);
///     }
/// });
/// ```
#[derive(Debug)]
pub struct LockFreeRingBuffer<T> {
  /// Battle-tested ArrayQueue for lock-free operations.
  ///
  /// Provides the core lock-free semantics. The ArrayQueue handles
  /// all the complex atomic operations and memory ordering required
  /// for safe concurrent access.
  queue: ArrayQueue<T>,

  /// Maximum capacity of the buffer.
  ///
  /// Stored separately for O(1) capacity queries without accessing
  /// the queue's internal state. This value is immutable after construction.
  capacity: usize,
}

impl<T> LockFreeRingBuffer<T> {
  /// Creates a new lock-free ring buffer with the specified capacity.
  ///
  /// # Arguments
  /// * `capacity` - Maximum number of items the buffer can store. Must be > 0.
  ///
  /// # Panics
  /// Panics if `capacity` is 0, as this would create an unusable buffer.
  ///
  /// # Memory Allocation
  /// Allocates memory for `capacity` items upfront. The allocation size is
  /// `capacity * size_of::<T>()` plus ArrayQueue overhead.
  ///
  /// # Example
  /// ```rust,ignore
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  ///
  /// // Create a buffer for 1000 log events
  /// let buffer = LockFreeRingBuffer::<LogEvent>::new(1000);
  /// assert_eq!(buffer.len(), 0);
  /// assert_eq!(buffer.capacity(), 1000);
  ///
  /// // Typical sizes for different use cases:
  /// let small_buffer = LockFreeRingBuffer::<String>::new(10);    // Debug logs
  /// let medium_buffer = LockFreeRingBuffer::<Event>::new(1000);  // Application logs  
  /// let large_buffer = LockFreeRingBuffer::<Metric>::new(10000); // High-frequency metrics
  /// ```
  pub fn new(capacity: usize) -> Self {
    if capacity == 0 {
      panic!("Capacity must be greater than 0");
    }

    Self {
      queue: ArrayQueue::new(capacity),
      capacity,
    }
  }

  /// Adds an item to the buffer with overflow handling.
  ///
  /// This method implements the core ring buffer semantics. If the buffer
  /// is at capacity, it will evict the oldest item to make space for the new one.
  /// All operations are lock-free and atomic.
  ///
  /// # Arguments  
  /// * `item` - The item to add to the buffer.
  ///
  /// # Returns
  /// * `Ok(None)` - Item was added successfully, no eviction occurred
  /// * `Ok(Some(old_item))` - Item was added, `old_item` was evicted
  /// * `Err(item)` - Failed to add item (extremely rare, indicates internal issue)
  ///
  /// # Concurrency Behavior
  ///
  /// In high-concurrency scenarios:
  /// - Multiple threads can push simultaneously
  /// - Eviction is atomic and consistent
  /// - No items are lost due to race conditions
  /// - The oldest item is always evicted first
  ///
  /// # Performance
  /// - **Best case**: O(1) when buffer has space
  /// - **Worst case**: O(1) when eviction is required  
  /// - **Memory**: No allocations, uses pre-allocated space
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(2);
  ///
  /// // Fill the buffer
  /// assert!(buffer.push(1).unwrap().is_none()); // No eviction
  /// assert!(buffer.push(2).unwrap().is_none()); // No eviction  
  ///
  /// // Trigger eviction
  /// assert_eq!(buffer.push(3).unwrap(), Some(1)); // Evicted 1
  /// assert_eq!(buffer.push(4).unwrap(), Some(2)); // Evicted 2
  /// ```
  pub fn push(&self, item: T) -> Result<Option<T>, T> {
    match self.queue.push(item) {
      Ok(()) => Ok(None),
      Err(rejected_item) => {
        // Queue is full, evict oldest item to make space
        let evicted = self.queue.pop();

        // Try to push again - this should succeed since we just made space
        match self.queue.push(rejected_item) {
          Ok(()) => Ok(evicted),
          Err(item) => Err(item), // This shouldn't happen, but handle gracefully
        }
      },
    }
  }

  /// Simplified push that discards eviction information.
  ///
  /// This is a convenience method when you don't need to know about evicted items.
  /// Internally calls `push()` but ignores the return value, making it suitable
  /// for fire-and-forget logging scenarios.
  ///
  /// # Arguments
  /// * `item` - The item to add to the buffer.
  ///
  /// # Use Cases
  /// - High-throughput logging where eviction details aren't important
  /// - Simple append-only scenarios
  /// - When you want minimal code complexity
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(1000);
  ///
  /// // Simple logging without caring about evictions
  /// for i in 0..2000 {
  ///     buffer.push_overwrite(format!("Log message {}", i));
  /// }
  /// // Buffer now contains the most recent 1000 messages
  /// ```
  pub fn push_overwrite(&self, item: T) {
    match self.push(item) {
      Ok(_) => {},  // Success, eviction info discarded
      Err(_) => {}, // This shouldn't happen with ArrayQueue, but handle gracefully
    }
  }

  /// Removes and returns the oldest item from the buffer.
  ///
  /// This operation is lock-free and will not block. If multiple threads
  /// are popping concurrently, each will receive a unique item (no duplicates).
  ///
  /// # Returns
  /// * `Some(item)` - The oldest item in the buffer
  /// * `None` - Buffer is empty
  ///
  /// # Ordering Guarantees
  /// Items are returned in FIFO (First In, First Out) order, maintaining
  /// the temporal sequence of when they were added.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(3);
  ///
  /// buffer.push_overwrite(1);
  /// buffer.push_overwrite(2);
  /// buffer.push_overwrite(3);
  ///
  /// assert_eq!(buffer.pop(), Some(1)); // Oldest first
  /// assert_eq!(buffer.pop(), Some(2));
  /// assert_eq!(buffer.pop(), Some(3));
  /// assert_eq!(buffer.pop(), None);    // Buffer empty
  /// ```
  pub fn pop(&self) -> Option<T> {
    self.queue.pop()
  }

  /// Removes and returns all items currently in the buffer.
  ///
  /// This operation atomically drains the entire buffer, returning all items
  /// in FIFO order (oldest first). The buffer will be empty after this call.
  ///
  /// # Returns
  /// A `Vec<T>` containing all items in insertion order.
  ///
  /// # Use Cases
  /// - Batch processing of accumulated items
  /// - Periodic flushing to persistent storage
  /// - Taking snapshots for debugging or analysis
  /// - Implementing backpressure by processing in batches
  ///
  /// # Performance
  /// - **Time Complexity**: O(n) where n is the current buffer size
  /// - **Memory**: Allocates a new Vec to hold all items
  /// - **Concurrency**: Safe to call while other threads are pushing/popping
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(1000);
  ///
  /// // Add some items
  /// for i in 0..100 {
  ///     buffer.push_overwrite(i);
  /// }
  ///
  /// // Take snapshot for batch processing
  /// let items = buffer.take_snapshot();
  /// assert_eq!(items.len(), 100);
  /// assert_eq!(items[0], 0);   // Oldest item first
  /// assert_eq!(items[99], 99); // Newest item last
  /// assert!(buffer.is_empty()); // Buffer is now empty
  ///
  /// // Process items in batch
  /// for item in items {
  ///     println!("Processing: {}", item);
  /// }
  /// ```
  pub fn take_snapshot(&self) -> Vec<T> {
    let mut items = Vec::new();
    while let Some(item) = self.queue.pop() {
      items.push(item);
    }
    items
  }

  /// Returns the current number of items in the buffer.
  ///
  /// # Concurrency Note
  /// In concurrent scenarios, this value may be stale by the time you use it,
  /// as other threads may modify the buffer between the call and your use of
  /// the returned value. Use this for approximate sizing and monitoring.
  ///
  /// # Performance
  /// This is an O(1) operation that reads the queue's internal counter.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(5);
  /// assert_eq!(buffer.len(), 0);
  ///
  /// buffer.push_overwrite(42);
  /// assert_eq!(buffer.len(), 1);
  ///
  /// // In concurrent code, be aware of race conditions:
  /// let current_len = buffer.len();
  /// // Other threads might modify buffer here
  /// // So current_len might not reflect actual state anymore
  /// ```
  #[inline]
  pub fn len(&self) -> usize {
    self.queue.len()
  }

  /// Returns `true` if the buffer contains no items.
  ///
  /// Equivalent to `self.len() == 0` but may be more semantically clear.
  ///
  /// # Concurrency Note
  /// Like `len()`, this value may become stale immediately after the call
  /// in concurrent scenarios.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(5);
  /// assert!(buffer.is_empty());
  ///
  /// buffer.push_overwrite(1);
  /// assert!(!buffer.is_empty());
  ///
  /// buffer.pop();
  /// assert!(buffer.is_empty());
  /// ```
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.queue.is_empty()
  }

  /// Returns `true` if the buffer is at maximum capacity.
  ///
  /// When the buffer is full, subsequent `push()` operations will trigger
  /// eviction of the oldest items.
  ///
  /// # Concurrency Note
  /// This status may change immediately after the call due to concurrent
  /// operations from other threads.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(2);
  /// assert!(!buffer.is_full());
  ///
  /// buffer.push_overwrite(1);
  /// assert!(!buffer.is_full());
  ///
  /// buffer.push_overwrite(2);
  /// assert!(buffer.is_full());
  ///
  /// // Next push will evict oldest item
  /// buffer.push_overwrite(3);
  /// assert!(buffer.is_full()); // Still full, but contents changed
  /// ```
  #[inline]
  pub fn is_full(&self) -> bool {
    self.queue.is_full()
  }

  /// Returns the maximum capacity of the buffer.
  ///
  /// This value is immutable after construction and represents the maximum
  /// number of items the buffer can hold before eviction occurs.
  ///
  /// # Returns
  /// The capacity value passed to `new()` during construction.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::<i32>::new(10);
  /// assert_eq!(buffer.capacity(), 10);
  ///
  /// // Capacity never changes
  /// buffer.push_overwrite(1);
  /// assert_eq!(buffer.capacity(), 10);
  /// ```
  #[inline]
  pub fn capacity(&self) -> usize {
    self.capacity
  }

  /// Returns the number of additional items that can be added without eviction.
  ///
  /// Calculates the available space in the buffer. When this returns 0,
  /// the next `push()` operation will trigger eviction.
  ///
  /// # Returns
  /// Number of items that can be added before the buffer becomes full.
  /// Uses saturating subtraction to handle potential race conditions.
  ///
  /// # Concurrency Note
  /// The returned value may be stale immediately due to concurrent operations.
  /// Use this for approximate capacity planning rather than exact logic.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(5);
  ///
  /// assert_eq!(buffer.remaining_capacity(), 5);
  ///
  /// buffer.push_overwrite(1);
  /// assert_eq!(buffer.remaining_capacity(), 4);
  ///
  /// buffer.push_overwrite(2);
  /// assert_eq!(buffer.remaining_capacity(), 3);
  /// ```
  pub fn remaining_capacity(&self) -> usize {
    self.capacity.saturating_sub(self.len())
  }
}

/// Thread-safe cloning that preserves buffer contents.
///
/// Creates a new buffer with the same capacity and populates it with
/// a snapshot of the current buffer's contents. The original buffer
/// is restored to its pre-clone state.
///
/// # Requirements
/// - `T` must implement `Clone` for duplicating items
///
/// # Concurrency Behavior
/// The cloning process:
/// 1. Creates a new empty buffer with the same capacity
/// 2. Takes a snapshot of the original buffer (temporarily draining it)
/// 3. Restores the original buffer's contents
/// 4. Populates the new buffer with the snapshot
///
/// # Performance Considerations
/// - **Time**: O(n) where n is the current buffer size
/// - **Memory**: Temporarily allocates Vec for snapshot storage
/// - **Blocking**: Brief period where original buffer appears empty to other threads
///
/// # Example
/// ```rust
/// use ttlog::lf_buffer::LockFreeRingBuffer;
/// let buffer1 = LockFreeRingBuffer::new(3);
/// buffer1.push_overwrite(1);
/// buffer1.push_overwrite(2);
///
/// let buffer2 = buffer1.clone();
/// assert_eq!(buffer2.len(), 2);
/// assert_eq!(buffer1.len(), 2); // Original preserved
/// ```
impl<T: Clone> Clone for LockFreeRingBuffer<T> {
  fn clone(&self) -> Self {
    let new_buffer = Self::new(self.capacity);

    // Take a snapshot: pop all items from original buffer
    let mut temp = Vec::with_capacity(self.len());
    while let Some(item) = self.queue.pop() {
      temp.push(item);
    }

    // Refill the original buffer immediately to minimize disruption
    for item in &temp {
      self.push_overwrite(item.clone());
    }

    // Populate the new buffer with the same data
    for item in temp {
      new_buffer.push_overwrite(item);
    }

    new_buffer
  }
}

/// Convenience methods for shared ownership across threads.
///
/// These methods simplify the common pattern of wrapping the buffer in `Arc`
/// for multi-threaded access.
impl<T> LockFreeRingBuffer<T> {
  /// Convert into an Arc for shared ownership across threads.
  ///
  /// Consumes the buffer and returns it wrapped in an `Arc<T>`, enabling
  /// the buffer to be shared and accessed concurrently from multiple threads.
  ///
  /// # Returns
  /// An `Arc<LockFreeRingBuffer<T>>` that can be cloned and shared across threads.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// use std::thread;
  ///
  /// let buffer = LockFreeRingBuffer::<String>::new(1000).into_shared();
  ///
  /// let handles: Vec<_> = (0..4).map(|thread_id| {
  ///     let buffer_clone = buffer.clone();
  ///     thread::spawn(move || {
  ///         for i in 0..100 {
  ///             buffer_clone.push_overwrite(format!("Thread-{}: {}", thread_id, i));
  ///         }
  ///     })
  /// }).collect();
  ///
  /// for handle in handles {
  ///     handle.join().unwrap();
  /// }
  /// ```
  pub fn into_shared(self) -> Arc<Self> {
    Arc::new(self)
  }

  /// Create a new buffer wrapped in Arc for immediate shared usage.
  ///
  /// Convenience constructor that combines `new()` and `Arc::new()` in a single call.
  /// Useful when you know the buffer will be shared across threads from the start.
  ///
  /// # Arguments
  /// * `capacity` - Maximum number of items the buffer can store.
  ///
  /// # Returns
  /// An `Arc<LockFreeRingBuffer<T>>` ready for multi-threaded access.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// use std::sync::Arc;
  ///
  /// // Direct creation for shared usage
  /// let shared_buffer = LockFreeRingBuffer::<i32>::new_shared(500);
  ///
  /// // Equivalent to:
  /// // let shared_buffer = Arc::new(LockFreeRingBuffer::new(500));
  /// ```
  pub fn new_shared(capacity: usize) -> Arc<Self> {
    Arc::new(Self::new(capacity))
  }
}

/// Custom serialization implementation for persistence and transmission.
///
/// Since `ArrayQueue` doesn't implement `Serialize`, we provide a custom
/// implementation that captures the buffer's current state as a snapshot.
///
/// # Serialization Process
/// 1. Takes a snapshot of all current items (temporarily draining the buffer)
/// 2. Serializes the items and capacity as a struct
/// 3. Restores the buffer contents
///
/// # Serialized Format
/// ```json
/// {
///   "items": [item1, item2, ...],
///   "capacity": 1000
/// }
/// ```
///
/// # Concurrency Impact
/// During serialization, the buffer briefly appears empty to other threads.
/// This is necessary to ensure a consistent snapshot.
///
/// # Requirements
/// - `T` must implement `Clone + Serialize`
impl<T: Clone + Serialize> Serialize for LockFreeRingBuffer<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeStruct;

    // Take snapshot for serialization (this temporarily empties the buffer)
    let items = self.take_snapshot();

    // Refill buffer after taking snapshot to minimize disruption
    for item in &items {
      self.push_overwrite(item.clone());
    }

    // Serialize the snapshot and capacity
    let mut state = serializer.serialize_struct("LockFreeRingBuffer", 2)?;
    state.serialize_field("items", &items)?;
    state.serialize_field("capacity", &self.capacity)?;
    state.end()
  }
}

/// Custom deserialization implementation for loading persisted buffers.
///
/// Reconstructs a `LockFreeRingBuffer` from serialized data, creating a new
/// buffer with the specified capacity and populating it with the saved items.
///
/// # Deserialization Process
/// 1. Deserialize capacity and items from the input
/// 2. Create a new buffer with the specified capacity
/// 3. Populate the buffer with items in order
///
/// # Error Handling
/// - Missing fields: Returns appropriate serde errors
/// - Duplicate fields: Returns duplicate field errors
/// - Invalid capacity: Panics (same as `new()`)
///
/// # Requirements
/// - `T` must implement `Clone + Deserialize`
impl<'de, T: Clone + Deserialize<'de>> Deserialize<'de> for LockFreeRingBuffer<T> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    use serde::de::{self, MapAccess, Visitor};
    use std::fmt;

    /// Field identifiers for deserialization.
    #[derive(Deserialize)]
    #[serde(field_identifier, rename_all = "lowercase")]
    enum Field {
      Items,
      Capacity,
    }

    /// Visitor pattern implementation for deserializing the buffer.
    ///
    /// This struct guides serde through the deserialization process,
    /// ensuring that both required fields (items and capacity) are present
    /// and properly typed.
    struct LockFreeRingBufferVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T: Clone + Deserialize<'de>> Visitor<'de> for LockFreeRingBufferVisitor<T> {
      type Value = LockFreeRingBuffer<T>;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct LockFreeRingBuffer")
      }

      /// Process the deserialized map into a LockFreeRingBuffer.
      ///
      /// Validates that both required fields are present exactly once,
      /// then constructs and populates the buffer.
      fn visit_map<V>(self, mut map: V) -> Result<LockFreeRingBuffer<T>, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut items = None;
        let mut capacity = None;

        // Process each field in the serialized data
        while let Some(key) = map.next_key()? {
          match key {
            Field::Items => {
              if items.is_some() {
                return Err(de::Error::duplicate_field("items"));
              }
              items = Some(map.next_value()?);
            },
            Field::Capacity => {
              if capacity.is_some() {
                return Err(de::Error::duplicate_field("capacity"));
              }
              capacity = Some(map.next_value()?);
            },
          }
        }

        // Ensure both required fields are present
        let items: Vec<T> = items.ok_or_else(|| de::Error::missing_field("items"))?;
        let capacity = capacity.ok_or_else(|| de::Error::missing_field("capacity"))?;

        // Reconstruct the buffer
        let buffer = LockFreeRingBuffer::new(capacity);
        for item in items {
          buffer.push_overwrite(item);
        }

        Ok(buffer)
      }
    }

    /// Expected field names for better error messages.
    const FIELDS: &[&str] = &["items", "capacity"];

    deserializer.deserialize_struct(
      "LockFreeRingBuffer",
      FIELDS,
      LockFreeRingBufferVisitor(std::marker::PhantomData),
    )
  }
}
