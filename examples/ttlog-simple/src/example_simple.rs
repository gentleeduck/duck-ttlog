use std::{ops::Deref, sync::Arc, thread};

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{debug, error, fatal, info, trace, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  let mut trace = Trace::init(2, 64, "test", Some("./tmp"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
  trace.set_level(ttlog::event::LogLevel::TRACE);

  // Step 2: Use standard tracing macros to log
  trace!("Application started successfully");
  debug!("Application started successfullyy");
  info!("Application started successfullyyy");
  warn!("Application started successfullyyyy");
  error!("An error occurred in the DB it might be shutting down");
  fatal!("An error occurred in the DB it might be shutting down");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  info!(user_id = user_id, username = username, "User logged in");

  // panic!("SIGINT received, shutting down!!");

  trace.shutdown();

  Ok(())
}

// struct Node {
//   parent: std::cell::RefCell<std::rc::Weak<Node>>,
//   name: String,
//   children: std::cell::RefCell<Vec<std::rc::Rc<Node>>>,
// }
//
// impl Node {
//   fn new(name: &str) -> std::rc::Rc<Node> {
//     std::rc::Rc::new(Node {
//       parent: std::cell::RefCell::new(std::rc::Weak::new()),
//       name: name.to_string(),
//       children: std::cell::RefCell::new(Vec::new()),
//     })
//   }
// }
//
// fn foo() {
//   let leaf = Node::new("leaf");
//   let branch = Node::new("branch");
//
//   *branch.parent.borrow_mut() = std::rc::Rc::downgrade(&leaf);
//   leaf.children.borrow_mut().push(branch);
// }
