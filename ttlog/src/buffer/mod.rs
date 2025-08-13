mod __test__;

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct TTlogBuffer<T: Clone> {
  pub buffer: Vec<Option<T>>,
  pub capacity: usize,
  pub head: usize,
}

impl<T: Clone> TTlogBuffer<T> {
  pub fn new(capacity: usize) -> Self {
    Self {
      buffer: vec![None; capacity],
      capacity,
      head: 0,
    }
  }

  pub fn push(&mut self, event: T) {
    if self.head == self.capacity - 1 {
      self.head = 0;
    } else if self.head < self.capacity - 1 {
      self.head += 1;
    }

    self.buffer[self.head] = Some(event);
  }

  pub fn iter(&self) -> impl Iterator<Item = &T> {
    (0..self.capacity).map(move |i| {
      let idx = (self.head + i + 1) % self.capacity;
      self.buffer[idx].as_ref().unwrap()
    })
  }
}
