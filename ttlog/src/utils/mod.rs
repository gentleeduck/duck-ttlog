pub fn current_thread_id_u32() -> u32 {
  use std::collections::hash_map::DefaultHasher;
  use std::hash::{Hash, Hasher};
  let mut hasher = DefaultHasher::new();
  std::thread::current().id().hash(&mut hasher);
  hasher.finish() as u32
}
