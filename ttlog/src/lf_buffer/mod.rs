mod __test__;

use crossbeam_queue::ArrayQueue;
use serde::{Deserialize, Serialize};
use std::sync::{
  atomic::{AtomicUsize, Ordering},
  Arc,
};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BufferStats {
  pub current_size: usize,
  pub capacity: usize,
  pub total_pushed: usize,
  pub total_evicted: usize,
  pub fill_ratio: f64,
}

#[derive(Debug)]
pub struct LockFreeRingBuffer<T> {
  /// Core lock-free queue
  queue: ArrayQueue<T>,
  /// Fast atomic counter (avoids queue.len() which can be expensive)
  count: AtomicUsize,
  /// Maximum capacity
  capacity: usize,
  /// Performance counters
  total_pushed: AtomicUsize,
  total_evicted: AtomicUsize,
}

impl<T> LockFreeRingBuffer<T> {
  pub fn new(capacity: usize) -> Self {
    if capacity == 0 {
      panic!("Capacity must be greater than 0");
    }

    Self {
      queue: ArrayQueue::new(capacity),
      capacity,
      count: AtomicUsize::new(0),
      total_pushed: AtomicUsize::new(0),
      total_evicted: AtomicUsize::new(0),
    }
  }

  #[inline]
  pub fn push(&self, item: T) -> Result<Option<T>, T> {
    match self.queue.push(item) {
      Ok(()) => {
        self.count.fetch_add(1, Ordering::Relaxed);
        self.total_pushed.fetch_add(1, Ordering::Relaxed);
        Ok(None)
      },
      Err(rejected_item) => {
        // Queue full - evict oldest
        let evicted = self.queue.pop();

        match self.queue.push(rejected_item) {
          Ok(()) => {
            self.total_pushed.fetch_add(1, Ordering::Relaxed);
            if evicted.is_some() {
              self.total_evicted.fetch_add(1, Ordering::Relaxed);
            } else {
              self.count.fetch_add(1, Ordering::Relaxed);
            }
            Ok(evicted)
          },
          Err(item) => Err(item), // This shouldn't happen
        }
      },
    }
  }

  #[inline]
  pub fn push_overwrite(&self, item: T) {
    match self.push(item) {
      Ok(_) => {},  // Success, eviction info discarded
      Err(_) => {}, // This shouldn't happen, but we handle it gracefully
    }
  }

  #[inline]
  pub fn pop(&self) -> Option<T> {
    match self.queue.pop() {
      Some(item) => {
        self.count.fetch_sub(1, Ordering::Relaxed);
        Some(item)
      },
      None => None,
    }
  }

  pub fn push_batch(&self, items: &mut Vec<T>) -> usize {
    let mut pushed = 0;

    for item in items.drain(..) {
      if self.push(item).is_ok() {
        pushed += 1;
      } else {
        break; // Stop on first failure
      }
    }

    pushed
  }

  pub fn pop_batch(&self, batch: &mut Vec<T>, max_items: usize) -> usize {
    let mut popped = 0;
    batch.clear();
    batch.reserve(max_items.min(self.len()));

    while popped < max_items {
      match self.pop() {
        Some(item) => {
          batch.push(item);
          popped += 1;
        },
        None => break,
      }
    }

    popped
  }

  pub fn take_snapshot(&self) -> Vec<T> {
    let estimated_size = self.len();
    let mut items = Vec::with_capacity(estimated_size);

    while let Some(item) = self.queue.pop() {
      items.push(item);
    }

    // Reset counter since we drained everything
    self.count.store(0, Ordering::Relaxed);

    items
  }

  #[inline]
  pub fn len(&self) -> usize {
    self.count.load(Ordering::Relaxed)
  }

  #[inline]
  pub fn is_empty(&self) -> bool {
    self.len() == 0
  }

  #[inline]
  pub fn is_full(&self) -> bool {
    self.len() >= self.capacity
  }

  #[inline]
  pub fn capacity(&self) -> usize {
    self.capacity
  }

  pub fn remaining_capacity(&self) -> usize {
    self.capacity.saturating_sub(self.len())
  }

  pub fn stats(&self) -> BufferStats {
    BufferStats {
      current_size: self.len(),
      capacity: self.capacity,
      total_pushed: self.total_pushed.load(Ordering::Relaxed),
      total_evicted: self.total_evicted.load(Ordering::Relaxed),
      fill_ratio: self.len() as f64 / self.capacity as f64,
    }
  }

  pub fn reset_stats(&self) {
    self.total_pushed.store(0, Ordering::Relaxed);
    self.total_evicted.store(0, Ordering::Relaxed);
  }

  pub fn try_reserve(&self, needed: usize) -> bool {
    let current = self.len();
    let available = self.capacity.saturating_sub(current);

    if available >= needed {
      return true; // Already have space
    }

    let to_evict = needed - available;
    let mut evicted = 0;

    // Evict items to make space
    while evicted < to_evict && !self.is_empty() {
      if self.queue.pop().is_some() {
        evicted += 1;
        self.count.fetch_sub(1, Ordering::Relaxed);
        self.total_evicted.fetch_add(1, Ordering::Relaxed);
      } else {
        break;
      }
    }

    evicted == to_evict
  }
}

// Convenience methods for shared ownership
impl<T> LockFreeRingBuffer<T> {
  pub fn into_shared(self) -> Arc<Self> {
    Arc::new(self)
  }

  pub fn new_shared(capacity: usize) -> Arc<Self> {
    Arc::new(Self::new(capacity))
  }
}

// Thread-safe cloning creates a new buffer with the same capacity
impl<T: Clone> Clone for LockFreeRingBuffer<T> {
  fn clone(&self) -> Self {
    let new_buffer = Self::new(self.capacity);

    // Snapshot current contents
    let items = self.take_snapshot();

    // Restore original buffer
    for item in &items {
      self.push_overwrite(item.clone());
    }

    // Populate new buffer
    for item in items {
      new_buffer.push_overwrite(item);
    }

    new_buffer
  }
}

// Custom serialization since ArrayQueue doesn't implement Serialize
impl<T: Clone + Serialize> Serialize for LockFreeRingBuffer<T> {
  fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
  where
    S: serde::Serializer,
  {
    use serde::ser::SerializeStruct;

    let items = self.take_snapshot();

    // Restore buffer after snapshot
    for item in &items {
      self.push_overwrite(item.clone());
    }

    let mut state = serializer.serialize_struct("LockFreeRingBuffer", 3)?;
    state.serialize_field("items", &items)?;
    state.serialize_field("capacity", &self.capacity)?;
    state.serialize_field("stats", &self.stats())?;
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
      Stats, // Optional field
    }

    struct BufferVisitor<T>(std::marker::PhantomData<T>);

    impl<'de, T: Clone + Deserialize<'de>> Visitor<'de> for BufferVisitor<T> {
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
            Field::Stats => {
              // Ignore stats during deserialization
              let _: BufferStats = map.next_value()?;
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

    const FIELDS: &[&str] = &["items", "capacity", "stats"];
    deserializer.deserialize_struct(
      "LockFreeRingBuffer",
      FIELDS,
      BufferVisitor(std::marker::PhantomData),
    )
  }
}
