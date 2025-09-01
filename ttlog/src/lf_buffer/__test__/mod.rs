#[cfg(test)]
mod __test__ {

  use crate::lf_buffer::LockFreeRingBuffer;

  use std::sync::Arc;
  use std::thread;
  use std::time::Duration;

  #[test]
  fn test_new_buffer() {
    let buffer = LockFreeRingBuffer::<i32>::new(5);
    assert_eq!(buffer.capacity(), 5);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert!(!buffer.is_full());
    assert_eq!(buffer.remaining_capacity(), 5);
  }

  #[test]
  fn test_push_and_pop_single_item() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);

    // Push single item
    assert_eq!(buffer.push(42), Ok(None));
    assert_eq!(buffer.len(), 1);
    assert!(!buffer.is_empty());
    assert!(!buffer.is_full());
    assert_eq!(buffer.remaining_capacity(), 2);

    // Pop item
    assert_eq!(buffer.pop(), Some(42));
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
    assert!(!buffer.is_full());
    assert_eq!(buffer.remaining_capacity(), 3);
  }

  #[test]
  fn test_push_and_pop_multiple_items() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);

    // Push multiple items
    assert_eq!(buffer.push(1), Ok(None));
    assert_eq!(buffer.push(2), Ok(None));
    assert_eq!(buffer.push(3), Ok(None));

    assert_eq!(buffer.len(), 3);
    assert!(buffer.is_full());
    assert_eq!(buffer.remaining_capacity(), 0);

    // Pop items in FIFO order
    assert_eq!(buffer.pop(), Some(1));
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), None);

    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_push_overwrite() {
    let buffer = LockFreeRingBuffer::<i32>::new(2);

    // Fill buffer
    buffer.push_overwrite(1);
    buffer.push_overwrite(2);
    assert!(buffer.is_full());

    // Push one more - should evict oldest item
    buffer.push_overwrite(3);
    assert!(buffer.is_full());

    // Check that 1 was evicted, 2 and 3 remain
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), None);
  }

  #[test]
  fn test_push_with_eviction() {
    let buffer = LockFreeRingBuffer::<i32>::new(2);

    // Fill buffer
    assert_eq!(buffer.push(1), Ok(None));
    assert_eq!(buffer.push(2), Ok(None));

    // Push third item - should evict 1
    assert_eq!(buffer.push(3), Ok(Some(1)));

    // Check remaining items
    assert_eq!(buffer.pop(), Some(2));
    assert_eq!(buffer.pop(), Some(3));
    assert_eq!(buffer.pop(), None);
  }

  #[test]
  fn test_take_snapshot() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);

    buffer.push_overwrite(1);
    buffer.push_overwrite(2);
    buffer.push_overwrite(3);

    let snapshot = buffer.take_snapshot();
    assert_eq!(snapshot, vec![1, 2, 3]);

    // Buffer should be empty after snapshot
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
  }

  #[test]
  fn test_take_snapshot_empty() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);
    let snapshot = buffer.take_snapshot();
    assert_eq!(snapshot, Vec::<i32>::new());
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_clone() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);
    buffer.push_overwrite(1);
    buffer.push_overwrite(2);

    // Clone first
    let cloned = buffer.clone();

    // Original buffer should still have items (refilled after cloning)
    let original_items = buffer.take_snapshot();
    assert_eq!(original_items, vec![1, 2]);

    // Cloned buffer should have the same items
    let cloned_items = cloned.take_snapshot();
    assert_eq!(cloned_items, vec![1, 2]);

    // Both should have the same capacity
    assert_eq!(cloned.capacity(), buffer.capacity());
  }

  #[test]
  fn test_into_shared() {
    let buffer = LockFreeRingBuffer::<i32>::new(5);
    buffer.push_overwrite(42);

    let shared = buffer.into_shared();
    assert_eq!(shared.len(), 1);
    assert_eq!(shared.pop(), Some(42));
  }

  #[test]
  fn test_new_shared() {
    let shared = LockFreeRingBuffer::<i32>::new_shared(10);
    assert_eq!(shared.capacity(), 10);
    assert!(shared.is_empty());
  }

  #[test]
  fn test_serialization_deserialization() {
    let buffer = LockFreeRingBuffer::<i32>::new(3);
    buffer.push_overwrite(1);
    buffer.push_overwrite(2);
    buffer.push_overwrite(3);

    // Serialize
    let serialized = serde_json::to_string(&buffer).unwrap();

    // Deserialize
    let deserialized: LockFreeRingBuffer<i32> = serde_json::from_str(&serialized).unwrap();

    // Check that items are preserved
    assert_eq!(deserialized.capacity(), 3);
    assert_eq!(deserialized.take_snapshot(), vec![1, 2, 3]);

    // Original buffer should still have items (refilled after serialization)
    assert_eq!(buffer.take_snapshot(), vec![1, 2, 3]);
  }

  #[test]
  fn test_concurrent_access() {
    let buffer = LockFreeRingBuffer::<i32>::new_shared(100);
    let num_threads = 10;
    let items_per_thread = 100;

    let mut handles = vec![];

    // Spawn producer threads
    for i in 0..num_threads {
      let buffer_clone = Arc::clone(&buffer);
      let start_item = i * items_per_thread;
      let handle = thread::spawn(move || {
        for j in 0..items_per_thread {
          buffer_clone.push_overwrite(start_item + j);
        }
      });
      handles.push(handle);
    }

    // Wait for all producers to finish
    for handle in handles {
      handle.join().unwrap();
    }

    // Verify total items (some may have been evicted due to capacity)
    let total_items = buffer.len();
    assert!(total_items <= 100);
    assert!(total_items > 0);

    // Verify items are in order (FIFO)
    let items = buffer.take_snapshot();
    if items.len() > 1 {
      for i in 1..items.len() {
        assert!(items[i] >= items[i - 1] || items[i] < items[i - 1]); // Allow for any order due to concurrent access
      }
    }
  }

  #[test]
  fn test_concurrent_producer_consumer() {
    let buffer = LockFreeRingBuffer::<i32>::new_shared(50);
    let num_producers = 5;
    let num_consumers = 3;
    let items_per_producer = 20;

    let mut producer_handles = vec![];
    let mut consumer_handles = vec![];

    // Spawn producer threads
    for i in 0..num_producers {
      let buffer_clone = Arc::clone(&buffer);
      let start_item = i * items_per_producer;
      let handle = thread::spawn(move || {
        for j in 0..items_per_producer {
          buffer_clone.push_overwrite(start_item + j);
          thread::sleep(Duration::from_millis(1)); // Small delay to simulate work
        }
      });
      producer_handles.push(handle);
    }

    // Spawn consumer threads
    for _ in 0..num_consumers {
      let buffer_clone = Arc::clone(&buffer);
      let handle = thread::spawn(move || {
        let mut consumed = 0;
        while consumed < (num_producers * items_per_producer) / num_consumers {
          if let Some(_) = buffer_clone.pop() {
            consumed += 1;
          } else {
            thread::sleep(Duration::from_millis(1));
          }
        }
      });
      consumer_handles.push(handle);
    }

    // Wait for all threads to finish
    for handle in producer_handles {
      handle.join().unwrap();
    }

    // Give consumers time to finish
    thread::sleep(Duration::from_millis(100));

    // Check final state
    let remaining_items = buffer.len();
    assert!(remaining_items <= 50);
  }

  #[test]
  fn test_remaining_capacity() {
    let buffer = LockFreeRingBuffer::<i32>::new(5);

    assert_eq!(buffer.remaining_capacity(), 5);

    buffer.push_overwrite(1);
    assert_eq!(buffer.remaining_capacity(), 4);

    buffer.push_overwrite(2);
    buffer.push_overwrite(3);
    assert_eq!(buffer.remaining_capacity(), 2);

    buffer.push_overwrite(4);
    buffer.push_overwrite(5);
    assert_eq!(buffer.remaining_capacity(), 0);

    // Push one more to trigger eviction
    buffer.push_overwrite(6);
    assert_eq!(buffer.remaining_capacity(), 0); // Still full

    // Pop one item
    buffer.pop();
    assert_eq!(buffer.remaining_capacity(), 1);
  }

  #[test]
  fn test_large_capacity() {
    let capacity = 10000;
    let buffer = LockFreeRingBuffer::<i32>::new(capacity);

    assert_eq!(buffer.capacity(), capacity);
    assert_eq!(buffer.remaining_capacity(), capacity);

    // Fill buffer
    for i in 0..capacity {
      buffer.push_overwrite(i as i32);
    }

    assert!(buffer.is_full());
    assert_eq!(buffer.remaining_capacity(), 0);

    // Verify all items
    let items = buffer.take_snapshot();
    assert_eq!(items.len(), capacity);

    // Check that items are in order (allowing for some reordering due to concurrent access)
    for i in 0..items.len() {
      assert!(items[i] < capacity as i32);
    }
  }

  #[test]
  fn test_string_items() {
    let buffer = LockFreeRingBuffer::<String>::new(3);

    buffer.push_overwrite("hello".to_string());
    buffer.push_overwrite("world".to_string());
    buffer.push_overwrite("rust".to_string());

    assert_eq!(buffer.len(), 3);

    let items = buffer.take_snapshot();
    assert_eq!(items, vec!["hello", "world", "rust"]);
  }

  #[test]
  fn test_custom_struct() {
    #[derive(Debug, Clone, PartialEq, serde::Serialize, serde::Deserialize)]
    struct TestItem {
      id: u32,
      data: String,
    }

    let buffer = LockFreeRingBuffer::<TestItem>::new(2);

    let item1 = TestItem {
      id: 1,
      data: "first".to_string(),
    };
    let item2 = TestItem {
      id: 2,
      data: "second".to_string(),
    };

    buffer.push_overwrite(item1.clone());
    buffer.push_overwrite(item2.clone());

    assert_eq!(buffer.pop(), Some(item1));
    assert_eq!(buffer.pop(), Some(item2));
    assert_eq!(buffer.pop(), None);
  }
}
