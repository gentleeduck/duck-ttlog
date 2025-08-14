mod __test__;

use serde::{Deserialize, Serialize};

use std::collections::VecDeque;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RingBuffer<T: Clone> {
  data: VecDeque<T>,
  capacity: usize,
}

impl<T: Clone> RingBuffer<T> {
  pub fn new(capacity: usize) -> Self {
    Self {
      data: VecDeque::with_capacity(capacity),
      capacity,
    }
  }

  pub fn push(&mut self, item: T) {
    if self.data.len() == self.capacity {
      self.data.pop_front();
    }
    self.data.push_back(item);
  }

  pub fn iter(&self) -> impl Iterator<Item = &T> {
    self.data.iter()
  }
}
