#[cfg(test)]
mod tests {
  use crate::{buffer::TTlogBuffer, event::Event};

  #[test]
  fn test_push_and_iter() {
    let capacity = 10;
    let mut buffer = TTlogBuffer::new(capacity);

    for i in 0..(capacity + 3) {
      buffer.push(Event {
        ts: 1755082651423,
        level: i as u8,
        message: format!("Event number {}", i),
      });
    }

    let items: Vec<_> = buffer.iter().collect();
    println!("{:#?}", items);

    assert_eq!(items.len(), capacity);
    assert_eq!(items.first().unwrap().level, 3);
    assert_eq!(items.last().unwrap().level, 12);
  }
}
