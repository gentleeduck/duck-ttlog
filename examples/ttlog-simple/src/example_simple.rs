use std::sync::Arc;

use ttlog::{
  file_listener::FileListener,
  trace::Trace,
  ttlog_macros::{error, info, warn},
};

pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
  println!("TTLog Quick Start Example");

  // Step 1: Initialize the tracing system
  let trace = Trace::init(4096, 64, "default", Some("/tmp/"));
  trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
  trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));

  // Step 2: Use standard tracing macros to log
  info!("Application started successfully");
  warn!("Something might be wrong in the session handler");
  error!("An error occurred in the DB it might be shutting down");

  // Step 3: Log with structured data
  let user_id = 42;
  let username = "alice";
  {
    const LEVEL: u8 = 2u8;
    const MESSAGE: &'static str = "User logged in";
    const MODULE: &'static str = module_path!();
    const FILE: &'static str = file!();
    const POSITION: (u32, u32) = (line!(), column!());
    const NUM_VALUES: usize = 1usize;
    static TARGET_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    static FILE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    static MESSAGE_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    static KV_ID: std::sync::OnceLock<u16> = std::sync::OnceLock::new();
    ttlog::trace::GLOBAL_LOGGER.with(|logger_cell| {
      if let Some(logger) = logger_cell.get() {
        if LEVEL <= logger.level.load(std::sync::atomic::Ordering::Relaxed) {
          let target_id = *TARGET_ID.get_or_init(|| logger.interner.intern_target(MODULE));
          let file_id = *FILE_ID.get_or_init(|| logger.interner.intern_file(FILE));
          struct SmallVec(smallvec::SmallVec<[u8; 128]>);

          impl SmallVec {
            pub fn with_capacity(cap: usize) -> Self {
              SmallVec(smallvec::SmallVec::with_capacity(cap))
            }
          }
          impl std::io::Write for SmallVec {
            fn write(&mut self, buf: &[u8]) -> std::io::Result<usize> {
              self.0.extend_from_slice(buf);
              Ok(buf.len())
            }
            fn flush(&mut self) -> std::io::Result<()> {
              Ok(())
            }
          }
          let mut buf = SmallVec::with_capacity(128);
          {
            use serde::ser::{SerializeMap, Serializer};
            let mut ser = serde_json::Serializer::new(&mut buf);
            let mut map = ser.serialize_map(Some(NUM_VALUES)).unwrap();
            let mut id_buf = itoa::Buffer::new();
            {
              map
                .serialize_entry(&username, &{
                  if let syn::Expr::Lit(expr_lit) = username {
                    match &expr_lit.lit {
                      syn::Lit::Str(_) => println!("This is a string literal"),
                      syn::Lit::Int(_) => println!("This is an integer literal"),
                      syn::Lit::Float(_) => println!("This is a float literal"),
                      _ => println!("Other literal"),
                    }
                  } else {
                    println!("Not a literal at all");
                  }
                })
                .unwrap()
            }
            map.end().unwrap();
          }
          let message_id = *MESSAGE_ID.get_or_init(|| logger.interner.intern_message(MESSAGE));
          let kv_id = *KV_ID.get_or_init(|| logger.interner.intern_kv(buf.0));
          logger.send_event_fast(
            LEVEL,
            target_id,
            Some(message_id),
            {
              static CACHED_THREAD_ID: std::sync::OnceLock<u8> = std::sync::OnceLock::new();
              *CACHED_THREAD_ID.get_or_init(|| ttlog::utils::current_thread_id_u32() as u8)
            },
            file_id,
            POSITION,
            Some(kv_id),
          );
        }
      }
    });
  };

  panic!("SIGINT received, shutting down!!");

  println!("Done! Check ./tmp/ for ttlog-*.bin files");
  println!("Run: ls -la ./tmp/ttlog-*.bin");

  Ok(())
}

// use std::sync::Arc;
//
// use ttlog::{
//   file_listener::FileListener,
//   trace::Trace,
//   ttlog_macros::{error, info, warn},
// };
//
// pub fn example_simple() -> Result<(), Box<dyn std::error::Error>> {
//   println!("TTLog Quick Start Example");
//
//   // Step 1: Initialize the tracing system
//   let trace = Trace::init(4096, 64, "default", Some("./tmp/"));
//   trace.add_listener(Arc::new(FileListener::new("./tmp/ttlog.log")?));
//   trace.add_listener(Arc::new(ttlog::stdout_listener::StdoutListener::new()));
//
//   // Step 2: Use standard tracing macros to log
//   info!("Application started successfully");
//   warn!("Something might be wrong in the session handler");
//   error!("An error occurred in the DB it might be shutting down");
//
//   // Step 3: Log with structured data
//   let user_id = 42;
//   let username = "alice";
//   info!(user_id = user_id, username = username, "User logged in");
//
//   panic!("SIGINT received, shutting down!!");
//
//   println!("Done! Check ./tmp/ for ttlog-*.bin files");
//   println!("Run: ls -la ./tmp/ttlog-*.bin");
//
//   Ok(())
// }
