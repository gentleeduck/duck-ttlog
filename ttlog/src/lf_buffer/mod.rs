mod __test__;

use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Maximum capacity allowed when constructing a `LockFreeRingBuffer` via the
/// fallible/`Deserialize` path. Guards against attacker-controlled snapshots
/// that would otherwise pre-allocate an unbounded amount of memory.
pub const MAX_RING_CAPACITY: usize = 1 << 20; // 1,048,576 entries

#[derive(Debug)]
pub struct LockFreeRingBuffer<T> {
  pub queue: ArrayQueue<T>,
  pub capacity: usize,
}

impl<T> LockFreeRingBuffer<T> {
  /// Constructs a ring buffer with a fixed `capacity`.
  ///
  /// # Panics
  ///
  /// Panics if `capacity` is `0` or exceeds [`MAX_RING_CAPACITY`]. This is a
  /// genuine programmer-error path: every attacker-reachable caller routes
  /// through [`LockFreeRingBuffer::new_checked`], which returns an error
  /// instead. Internal callers pass trusted, statically valid capacities.
  pub fn new(capacity: usize) -> Self {
    Self::new_checked(capacity).expect("LockFreeRingBuffer::new called with invalid capacity")
  }

  /// Checked constructor used by `Deserialize` and other untrusted-input paths.
  /// Rejects zero and capacities exceeding [`MAX_RING_CAPACITY`].
  pub fn new_checked(capacity: usize) -> Result<Self, &'static str> {
    if capacity == 0 {
      return Err("ring capacity must be > 0");
    }
    if capacity > MAX_RING_CAPACITY {
      return Err("ring capacity exceeds limit");
    }
    Ok(Self {
      queue: ArrayQueue::new(capacity),
      capacity,
    })
  }

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

  pub fn push_overwrite(&self, item: T) {
    let _ = self.push(item);
  }

  pub fn pop(&self) -> Option<T> {
    self.queue.pop()
  }

  pub fn take_snapshot(&self) -> Vec<T> {
    let mut items = Vec::new();
    while let Some(item) = self.queue.pop() {
      items.push(item);
    }
    items
  }

  #[inline]
  pub fn len(&self) -> usize {
    self.queue.len()
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    self.queue.is_empty()
  }

  #[inline]
  pub fn is_full(&self) -> bool {
    self.queue.is_full()
  }

  #[inline]
  pub fn capacity(&self) -> usize {
    self.capacity
  }

  pub fn remaining_capacity(&self) -> usize {
    self.capacity.saturating_sub(self.len())
  }
}

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

impl<T> LockFreeRingBuffer<T> {
  pub fn into_shared(self) -> Arc<Self> {
    Arc::new(self)
  }

  pub fn new_shared(capacity: usize) -> Arc<Self> {
    Arc::new(Self::new(capacity))
  }
}

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

        // Reconstruct the buffer with bounded capacity to prevent
        // attacker-controlled OOM via crafted snapshots.
        let buffer = LockFreeRingBuffer::new_checked(capacity).map_err(serde::de::Error::custom)?;
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
