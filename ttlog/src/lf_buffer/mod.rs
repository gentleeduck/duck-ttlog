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
/// # Type Parameters
/// * `T` - The type of items stored in the buffer.
#[derive(Debug)]
pub struct LockFreeRingBuffer<T> {
  /// Battle-tested ArrayQueue for lock-free operations
  queue: ArrayQueue<T>,
  /// Maximum capacity of the buffer
  capacity: usize,
}

impl<T> LockFreeRingBuffer<T> {
  /// Creates a new lock-free ring buffer with the specified capacity.
  ///
  /// # Arguments
  /// * `capacity` - Maximum number of items the buffer can store.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::<i32>::new(10);
  ///  assert_eq!(buffer.len(), 0);
  ///  assert_eq!(buffer.capacity(), 10);
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

  /// Adds an item to the buffer.
  ///
  /// If the buffer is at capacity, this will remove the oldest item
  /// to make space. This operation is lock-free and wait-free.
  ///
  /// # Arguments  
  /// * `item` - The item to add to the buffer.
  ///
  /// # Returns
  /// * `Ok(None)` - Item was added successfully, no eviction occurred
  /// * `Ok(Some(old_item))` - Item was added, `old_item` was evicted
  /// * `Err(item)` - Failed to add item (shouldn't happen in normal usage)
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(2);
  ///
  /// assert!(buffer.push(1).unwrap().is_none()); // No eviction
  /// assert!(buffer.push(2).unwrap().is_none()); // No eviction  
  /// assert_eq!(buffer.push(3).unwrap(), Some(1)); // Evicted 1
  /// ```
  pub fn push(&self, item: T) -> Result<Option<T>, T> {
    match self.queue.push(item) {
      Ok(()) => Ok(None),
      Err(rejected_item) => {
        // Queue is full, evict oldest item
        let evicted = self.queue.pop();

        // Try to push again - this should succeed
        match self.queue.push(rejected_item) {
          Ok(()) => Ok(evicted),
          Err(item) => Err(item), // This shouldn't happen, but handle gracefully
        }
      },
    }
  }

  /// Simplified push that discards eviction information.
  ///
  /// This is a convenience method when you don't care about evicted items.
  ///
  /// # Arguments
  /// * `item` - The item to add to the buffer.
  pub fn push_overwrite(&self, item: T) {
    match self.push(item) {
      Ok(_) => {},  // Success, eviction info discarded
      Err(_) => {}, // This shouldn't happen, but we handle it gracefully
    }
  }

  /// Removes and returns the oldest item from the buffer.
  ///
  /// # Returns
  /// * `Some(item)` - The oldest item in the buffer
  /// * `None` - Buffer is empty
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(3);
  /// buffer.push_overwrite(1);
  /// buffer.push_overwrite(2);
  ///
  /// assert_eq!(buffer.pop(), Some(1));
  /// assert_eq!(buffer.pop(), Some(2));
  /// assert_eq!(buffer.pop(), None);
  /// ```
  pub fn pop(&self) -> Option<T> {
    self.queue.pop()
  }

  /// Removes and returns all items currently in the buffer.
  ///
  /// Items are returned in FIFO order (oldest first).
  /// The buffer will be empty after this operation.
  ///
  /// # Returns
  /// A `Vec<T>` containing all items in insertion order.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(3);
  /// buffer.push_overwrite(1);
  /// buffer.push_overwrite(2);
  /// buffer.push_overwrite(3);
  ///
  /// let items = buffer.take_snapshot();
  /// assert_eq!(items, vec![1, 2, 3]);
  /// assert!(buffer.is_empty());
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
  /// Note: In concurrent scenarios, this value may be stale by the time
  /// you use it, as other threads may modify the buffer.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(5);
  /// assert_eq!(buffer.len(), 0);
  ///
  /// buffer.push_overwrite(42);
  /// assert_eq!(buffer.len(), 1);
  /// ```
  #[inline]
  pub fn len(&self) -> usize {
    self.queue.len()
  }

  /// Returns `true` if the buffer contains no items.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(5);
  /// assert!(buffer.is_empty());
  ///
  /// buffer.push_overwrite(1);
  /// assert!(!buffer.is_empty());
  /// ```
  #[inline]
  pub fn is_empty(&self) -> bool {
    self.queue.is_empty()
  }

  /// Returns `true` if the buffer is at maximum capacity.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::new(2);
  /// assert!(!buffer.is_full());
  ///
  /// buffer.push_overwrite(1);
  /// buffer.push_overwrite(2);
  /// assert!(buffer.is_full());
  /// ```
  #[inline]
  pub fn is_full(&self) -> bool {
    self.queue.is_full()
  }

  /// Returns the maximum capacity of the buffer.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::<i32>::new(10);
  /// assert_eq!(buffer.capacity(), 10);
  /// ```
  #[inline]
  pub fn capacity(&self) -> usize {
    self.capacity
  }

  /// Attempts to reserve space for additional items.
  ///
  /// Since ArrayQueue has fixed capacity, this will return the number
  /// of items that can still be added before the buffer is full.
  ///
  /// # Returns
  /// Number of additional items that can be added without eviction.
  pub fn remaining_capacity(&self) -> usize {
    self.capacity.saturating_sub(self.len())
  }
}

// Thread-safe cloning creates a new buffer with the same capacity
impl<T: Clone> Clone for LockFreeRingBuffer<T> {
  fn clone(&self) -> Self {
    let new_buffer = Self::new(self.capacity);

    // Take a snapshot: pop all items
    let mut temp = Vec::with_capacity(self.len());
    while let Some(item) = self.queue.pop() {
      temp.push(item);
    }

    // Refill the original buffer immediately
    for item in &temp {
      self.push_overwrite(item.clone());
    }

    // Populate the new buffer
    for item in temp {
      new_buffer.push_overwrite(item);
    }

    new_buffer
  }
}

// Convenience methods for shared ownership
impl<T> LockFreeRingBuffer<T> {
  /// Convert into an Arc for shared ownership across threads.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let buffer = LockFreeRingBuffer::<i32>::new(10);
  /// let shared_buffer = buffer.into_shared();
  /// // Now can be shared across threads
  /// ```
  pub fn into_shared(self) -> Arc<Self> {
    Arc::new(self)
  }

  /// Create a new buffer wrapped in Arc for immediate shared usage.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::lf_buffer::LockFreeRingBuffer;
  /// let shared_buffer = LockFreeRingBuffer::<i32>::new_shared(10);
  /// // Ready to use across multiple threads
  /// ```
  pub fn new_shared(capacity: usize) -> Arc<Self> {
    Arc::new(Self::new(capacity))
  }
}

// Custom serialization since ArrayQueue doesn't implement Serialize
impl<T: Clone + Serialize> Serialize for LockFreeRingBuffer<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeStruct;

    // Take snapshot for serialization (this empties the buffer)
    let items = self.take_snapshot();

    // Refill buffer after taking snapshot
    for item in &items {
      self.push_overwrite(item.clone());
    }

    let mut state = serializer.serialize_struct("LockFreeRingBuffer", 2)?;
    state.serialize_field("items", &items)?;
    state.serialize_field("capacity", &self.capacity)?;
    state.end()
  }
}

// Custom deserialization
impl<'de, T: Clone + Deserialize<'de>> Deserialize<'de> for LockFreeRingBuffer<T> {
  fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
  where
    D: serde::Deserializer<'de>,
  {
    use serde::de::{self, MapAccess, Visitor};
    use std::fmt;

    #[derive(Deserialize)]
    #[serde(field_identifier, rename_all = "lowercase")]
    enum Field {
      Items,
      Capacity,
    }

    struct LockFreeRingBufferVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T: Clone + Deserialize<'de>> Visitor<'de> for LockFreeRingBufferVisitor<T> {
      type Value = LockFreeRingBuffer<T>;

      fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
        formatter.write_str("struct LockFreeRingBuffer")
      }

      fn visit_map<V>(self, mut map: V) -> Result<LockFreeRingBuffer<T>, V::Error>
      where
        V: MapAccess<'de>,
      {
        let mut items = None;
        let mut capacity = None;

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

        let items: Vec<T> = items.ok_or_else(|| de::Error::missing_field("items"))?;
        let capacity = capacity.ok_or_else(|| de::Error::missing_field("capacity"))?;

        let buffer = LockFreeRingBuffer::new(capacity);
        for item in items {
          buffer.push_overwrite(item);
        }

        Ok(buffer)
      }
    }

    const FIELDS: &[&str] = &["items", "capacity"];
    deserializer.deserialize_struct(
      "LockFreeRingBuffer",
      FIELDS,
      LockFreeRingBufferVisitor(std::marker::PhantomData),
    )
  }
}
