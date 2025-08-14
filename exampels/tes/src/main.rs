use crossbeam_channel::{select, unbounded, Receiver, Sender};
use std::thread;
use std::time::Duration;

// Message enum to send different kinds of data
enum Message {
  Data(i32),
  Quit,
}

fn producer(id: usize, tx: Sender<Message>) {
  for i in 0..5 {
    println!("Producer {} sending {}", id, i);
    tx.send(Message::Data(i)).unwrap();
    thread::sleep(Duration::from_millis(100 * id as u64));
  }
  println!("Producer {} sending Quit", id);
  tx.send(Message::Quit).unwrap();
}

fn consumer(id: usize, rx: Receiver<Message>) {
  loop {
    match rx.recv() {
      Ok(Message::Data(value)) => println!("Consumer {} received {}", id, value),
      Ok(Message::Quit) => {
        println!("Consumer {} quitting", id);
        break;
      },
      Err(_) => break, // channel closed
    }
  }
}

fn main() {
  let (tx, rx) = unbounded::<Message>();

  // Spawn multiple producers
  for i in 1..=3 {
    let tx_clone = tx.clone();
    thread::spawn(move || producer(i, tx_clone));
  }

  // Spawn multiple consumers
  let mut consumers = Vec::new();
  for i in 1..=2 {
    let rx_clone = rx.clone();
    let handle = thread::spawn(move || consumer(i, rx_clone));
    consumers.push(handle);
  }

  // Main thread can also listen and handle messages using `select!`
  for _ in 0..3 {
    select! {
        recv(rx) -> msg => match msg {
            Ok(Message::Data(value)) => println!("Main thread got {}", value),
            Ok(Message::Quit) => println!("Main thread received Quit"),
            Err(_) => break,
        }
    }
  }

  // Wait for all consumer threads to finish
  for handle in consumers {
    handle.join().unwrap();
  }

  println!("All threads finished!");
}
