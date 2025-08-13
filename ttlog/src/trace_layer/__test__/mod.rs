#[cfg(test)]
mod tests {
  use crate::buffer::RingBuffer;
  use crate::trace_layer::BufferLayer;

  use std::sync::{Arc, Mutex};
  use tracing::{info, Dispatch};
  use tracing_subscriber::layer::SubscriberExt;
  use tracing_subscriber::Registry;

  #[test]
  fn single_event_pushes_to_buffer() {
    let buffer = Arc::new(Mutex::new(RingBuffer::new(10)));
    let layer = BufferLayer::new(buffer.clone());

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::with_default(subscriber, || {
      info!("Hello world");
    });

    let buf = buffer.lock().unwrap();
    assert_eq!(buf.iter().collect::<Vec<_>>().len(), 1);
    assert_eq!(buf.iter().next().unwrap().message, "Hello world");
  }

  #[test]
  fn concurrent_logging() {
    use std::sync::Arc;
    use std::thread;
    use tracing::{dispatcher, info};
    use tracing_subscriber::{layer::SubscriberExt, Registry};

    let capacity = 1000;
    let buffer = Arc::new(Mutex::new(RingBuffer::new(capacity)));
    let layer = BufferLayer::new(buffer.clone());

    // Wrap subscriber in Arc so threads can share it
    let subscriber = Arc::new(Registry::default().with(layer));

    let threads: Vec<_> = (0..10)
      .map(|t| {
        let disp = Dispatch::from(subscriber.clone());
        thread::spawn(move || {
          // Each thread sets the subscriber locally
          dispatcher::with_default(&disp, || {
            for i in 0..500 {
              info!("Thread {} - {}", t, i);
            }
          });
        })
      })
      .collect();

    for t in threads {
      t.join().unwrap();
    }

    // Lock buffer and check results
    let buf = buffer.lock().unwrap();
    assert_eq!(buf.iter().collect::<Vec<_>>().len(), capacity);

    // Optional: print first 5 events to verify
    // for event in buf.iter().take(5) {
    //   println!("{:?}", event);
    // }
  }

  #[test]
  fn empty_message_logged() {
    let buffer = Arc::new(Mutex::new(RingBuffer::new(5)));
    let layer = BufferLayer::new(buffer.clone());

    let subscriber = Registry::default().with(layer);
    tracing::subscriber::with_default(subscriber, || {
      info!("");
    });

    let buf = buffer.lock().unwrap();
    assert_eq!(buf.iter().collect::<Vec<_>>().len(), 1);
    assert_eq!(buf.iter().next().unwrap().message, "");
  }
}
