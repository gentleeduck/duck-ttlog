#[cfg(test)]
mod tests {
  use crate::{buffer::RingBuffer, event::Event};

  #[test]
  fn test_push_and_iter() {
    let capacity = 10;
    let mut buffer = RingBuffer::new(capacity);

    for i in 0..(capacity + 3) {
      buffer.push(Event {
        timestamps: 1755082651423,
        level: i.to_string(),
        message: format!("Event number {}", i),
      });
    }

    let items: Vec<_> = buffer.iter().collect();

    assert_eq!(items.len(), capacity);
    assert_eq!(items.first().unwrap().level, 3.to_string());
    assert_eq!(items.last().unwrap().level, 12.to_string());
  }
}
