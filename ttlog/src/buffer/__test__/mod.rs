#[cfg(test)]
mod __test__ {

  use crate::buffer::RingBuffer;
  use crate::event::LogEvent;

  #[test]
  fn test_ring_buffer_new() {
    let buffer: RingBuffer<i32> = RingBuffer::new(10);
    assert_eq!(buffer.capacity, 10);
    assert_eq!(buffer.len(), 0);
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_ring_buffer_push_single() {
    let mut buffer = RingBuffer::new(5);
    buffer.push(42);

    assert_eq!(buffer.len(), 1);
    assert!(!buffer.is_empty());

    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert_eq!(items, vec![42]);
  }

  #[test]
  fn test_ring_buffer_push_multiple() {
    let mut buffer = RingBuffer::new(3);

    buffer.push(1);
    buffer.push(2);
    buffer.push(3);

    assert_eq!(buffer.len(), 3);
    assert_eq!(buffer.capacity, 3);

    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert_eq!(items, vec![1, 2, 3]);
  }

  #[test]
  fn test_ring_buffer_overflow() {
    let mut buffer = RingBuffer::new(3);

    buffer.push(1);
    buffer.push(2);
    buffer.push(3);
    buffer.push(4); // should evict 1

    assert_eq!(buffer.len(), 3);
    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert_eq!(items, vec![2, 3, 4]);

    buffer.push(5); // should evict 2
    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert_eq!(items, vec![3, 4, 5]);
  }

  #[test]
  fn test_ring_buffer_take_snapshot() {
    let mut buffer = RingBuffer::new(5);

    buffer.push(10);
    buffer.push(20);
    buffer.push(30);

    let snapshot = buffer.take_snapshot();
    assert_eq!(snapshot, vec![10, 20, 30]);
    assert!(buffer.is_empty());
    assert_eq!(buffer.len(), 0);
    assert_eq!(buffer.capacity, 5);
  }

  #[test]
  fn test_ring_buffer_take_snapshot_empty() {
    let mut buffer: RingBuffer<i32> = RingBuffer::new(5);

    let snapshot = buffer.take_snapshot();
    assert!(snapshot.is_empty());
    assert!(buffer.is_empty());
  }

  #[test]
  fn test_ring_buffer_iter() {
    let mut buffer = RingBuffer::new(4);

    buffer.push(1);
    buffer.push(2);
    buffer.push(3);

    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert_eq!(items, vec![1, 2, 3]);
  }

  #[test]
  fn test_ring_buffer_iter_empty() {
    let buffer: RingBuffer<i32> = RingBuffer::new(5);
    let items: Vec<i32> = buffer.iter().cloned().collect();
    assert!(items.is_empty());
  }

  #[test]
  fn test_ring_buffer_with_events() {
    let mut buffer = RingBuffer::new(3);

    let event1 = LogEvent::new(
      1000,
      "INFO".to_string(),
      "First".to_string(),
      "target1".to_string(),
    );
    let event2 = LogEvent::new(
      2000,
      "WARN".to_string(),
      "Second".to_string(),
      "target2".to_string(),
    );
    let event3 = LogEvent::new(
      3000,
      "ERROR".to_string(),
      "Third".to_string(),
      "target3".to_string(),
    );

    buffer.push(event1.clone());
    buffer.push(event2.clone());
    buffer.push(event3.clone());

    let events: Vec<LogEvent> = buffer.iter().cloned().collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].message, "First");
    assert_eq!(events[1].message, "Second");
    assert_eq!(events[2].message, "Third");

    // Test overflow with events
    let event4 = LogEvent::new(
      4000,
      "DEBUG".to_string(),
      "Fourth".to_string(),
      "target4".to_string(),
    );
    buffer.push(event4.clone());
    let events: Vec<LogEvent> = buffer.iter().cloned().collect();
    assert_eq!(events.len(), 3);
    assert_eq!(events[0].message, "Second"); // First evicted
    assert_eq!(events[2].message, "Fourth");
  }

  // test: push and snapshot repeatedly
  #[test]
  fn test_ring_buffer_push_and_snapshot_repeatedly() {
    let mut buffer = RingBuffer::new(2);

    buffer.push(1);
    let snap1 = buffer.take_snapshot();
    assert_eq!(snap1, vec![1]);
    assert!(buffer.is_empty());

    buffer.push(2);
    buffer.push(3); // should evict 2 if buffer was filled previously
    let snap2 = buffer.take_snapshot();
    assert_eq!(snap2, vec![2, 3]);
    assert!(buffer.is_empty());
  }
}
