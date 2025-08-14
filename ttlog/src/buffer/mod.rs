mod __test__;

use serde::{Deserialize, Serialize};
use std::collections::VecDeque;

/// A fixed-capacity ring buffer that stores items in insertion order.
///
/// When the buffer reaches its capacity, adding a new item
/// will automatically evict the oldest item.
///
/// # Type Parameters
/// * `T` - The type of the items stored in the buffer. Must implement `Clone`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingBuffer<T: Clone> {
  /// Internal storage for the buffer
  data: VecDeque<T>,

  /// Maximum number of items the buffer can hold
  capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
  /// Creates a new empty ring buffer with the specified capacity.
  ///
  /// # Arguments
  /// * `capacity` - The maximum number of items the buffer can store.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let buffer: RingBuffer<i32> = RingBuffer::new(10);
  /// assert_eq!(buffer.len(), 0);
  /// ```
  pub fn new(capacity: usize) -> Self {
    Self {
      data: VecDeque::with_capacity(capacity),
      capacity,
    }
  }

  /// Adds a new item to the buffer.
  ///
  /// If the buffer is already at capacity, the oldest item is removed
  /// to make space for the new item.
  ///
  /// # Arguments
  /// * `item` - The item to add to the buffer.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let mut buffer = RingBuffer::new(2);
  /// buffer.push(1);
  /// buffer.push(2);
  /// buffer.push(3); // evicts 1
  /// assert_eq!(buffer.len(), 2);
  /// ```
  pub fn push(&mut self, item: T) {
    if self.data.len() == self.capacity {
      self.data.pop_front();
    }
    self.data.push_back(item);
  }

  /// Removes and returns all items currently in the buffer.
  ///
  /// This operation leaves the buffer empty but preserves its capacity,
  /// avoiding reallocations on future pushes.
  ///
  /// # Returns
  /// A `Vec<T>` containing all items in insertion order.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let mut buffer = RingBuffer::new(3);
  /// buffer.push(1);
  /// buffer.push(2);
  /// let snapshot = buffer.take_snapshot();
  /// assert_eq!(snapshot, vec![1, 2]);
  /// assert!(buffer.is_empty());
  /// ```
  pub fn take_snapshot(&mut self) -> Vec<T> {
    let old = std::mem::replace(&mut self.data, VecDeque::with_capacity(self.capacity));
    old.into_iter().collect()
  }

  /// Returns an iterator over the items currently in the buffer.
  ///
  /// Items are iterated in insertion order (oldest to newest).
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let mut buffer = RingBuffer::new(2);
  /// buffer.push(10);
  /// buffer.push(20);
  /// for item in buffer.iter() {
  ///     println!("{}", item);
  /// }
  /// ```
  pub fn iter(&self) -> impl Iterator<Item = &T> {
    self.data.iter()
  }

  /// Returns the number of items currently in the buffer.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let mut buffer = RingBuffer::new(2);
  /// assert_eq!(buffer.len(), 0);
  /// buffer.push(5);
  /// assert_eq!(buffer.len(), 1);
  /// ```
  pub fn len(&self) -> usize {
    self.data.len()
  }

  /// Returns `true` if the buffer is empty.
  ///
  /// # Example
  /// ```rust
  /// use ttlog::buffer::RingBuffer;
  ///
  /// let buffer: RingBuffer<i32> = RingBuffer::new(2);
  /// assert!(buffer.is_empty());
  /// ```
  pub fn is_empty(&self) -> bool {
    self.data.is_empty()
  }
}
