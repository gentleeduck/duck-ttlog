mod snapshot_read;
mod utils;

// use crate::snapshot_read::SnapshotFile;
// use crate::utils::{generate_ascii_art, print_snapshots};
// use colored::*;
// use inquire::Select;
// use std::error::Error;
// use std::fs;

// fn main() -> Result<(), Box<dyn Error>> {
//   // Clear screen
//   print!("\x1B[2J\x1B[1;1H");
//
//   // ASCII banner
//   let banner = generate_ascii_art("TTLOG")?;
//   println!("{}", banner.bright_yellow().bold());
//
//   loop {
//     // Main menu
//     let choice = Select::new(
//       "Main Menu - Select an action:",
//       vec!["Show All Files", "Preview All Files", "Exit"],
//     )
//     .prompt()?;
//
//     match choice {
//       "Show All Files" => show_all_files_menu()?,
//       "Preview All Files" => preview_all_files()?,
//       "Exit" => {
//         println!("{}", "Goodbye!".red().bold());
//         break;
//       },
//       _ => unreachable!(),
//     }
//   }
//
//   Ok(())
// }
//
// fn show_all_files_menu() -> Result<(), Box<dyn Error>> {
//   let files = snapshot_read::read_snapshots()?;
//   if files.is_empty() {
//     println!("{}", "No log files found.".red());
//     return Ok(());
//   }
//
//   loop {
//     let mut options: Vec<String> = files
//       .iter()
//       .map(|f| f.name.clone()) // Convert PathBuf to String
//       .collect();
//
//     options.push("Back".to_string());
//
//     let choice = Select::new("Select a file to manage:", options).prompt()?;
//
//     if choice == "Back" {
//       break;
//     }
//
//     file_action_menu(&choice, &files)?;
//   }
//
//   Ok(())
// }
//
// fn file_action_menu(file: &str, snapshots: &Vec<SnapshotFile>) -> Result<(), Box<dyn Error>> {
//   loop {
//     match Select::new(
//       &format!("File: {} - Choose an action:", file),
//       vec!["Preview", "Delete", "Back"],
//     )
//     .prompt()?
//     {
//       "Preview" => preview_file(file, snapshots)?,
//       "Delete" => {
//         fs::remove_file(format!("/tmp/{}.bin", file))?;
//         println!("{}", format!("Deleted file: {}", file).red());
//         break; // exit after deletion
//       },
//       "Back" => break,
//       _ => unreachable!(),
//     }
//   }
//   Ok(())
// }
//
// fn preview_file(file: &str, snapshots: &Vec<SnapshotFile>) -> Result<(), Box<dyn Error>> {
//   // Find snapshot by path
//   if let Some(snapshot) = snapshots.iter().find(|s| s.name == file) {
//     println!("=== Preview: {} ===", snapshot.name);
//
//     // Instead of raw debug output, reuse your old rendering logic
//     print_snapshots(&vec![snapshot.clone()]);
//     // Or if you had a function like render_snapshot(snapshot), call that:
//     // render_snapshot(snapshot);
//   } else {
//     println!("File '{}' not found in snapshots.", file);
//   }
//
//   Ok(())
// }
//
// fn preview_all_files() -> Result<(), Box<dyn Error>> {
//   let snapshots = snapshot_read::read_snapshots()?;
//   print_snapshots(&snapshots);
//   Ok(())
// }

/// Macro to create a vector of bytes
// macro_rules! wildduck {
//   ( $( $x:expr ),* ) => {
//     {
//       let mut temp = Vec::<_>::new();
//       $(
//           temp.push($x);
//       )*
//       temp
//     }
//   };
// }
use std::thread;
use std::time::Duration;

use criterion::{criterion_group, criterion_main, BenchmarkId, Criterion};

use ttlog::trace::Trace;

fn bench_single_threaded_logging(c: &mut Criterion) {
  let trace_system = Trace::init(1024, 128);

  c.bench_function("single_log_event", |b| {
    b.iter(|| {
      tracing::info!("Benchmark log message {}", 42);
    })
  });
}

fn bench_multi_threaded_logging(c: &mut Criterion) {
  let mut group = c.benchmark_group("multithreaded_logging");

  for thread_count in [1, 2, 4, 8, 16].iter() {
    group.bench_with_input(
      BenchmarkId::new("threads", thread_count),
      thread_count,
      |b, &thread_count| {
        b.iter(|| {
          let trace_system = Trace::init(1024 * thread_count, 128);
          let handles: Vec<_> = (0..thread_count)
            .map(|i| {
              thread::spawn(move || {
                for j in 0..1000 {
                  tracing::info!("Thread {} log {}", i, j);
                }
              })
            })
            .collect();

          for handle in handles {
            handle.join().unwrap();
          }
        })
      },
    );
  }
  group.finish();
}

fn bench_structured_logging(c: &mut Criterion) {
  let trace_system = Trace::init(1024, 128);

  c.bench_function("structured_log", |b| {
    b.iter(|| {
      tracing::info!(
        user_id = 12345,
        action = "login",
        duration_ms = 150,
        "User login completed"
      );
    })
  });
}

fn bench_log_levels(c: &mut Criterion) {
  let trace_system = Trace::init(1024, 128);
  let mut group = c.benchmark_group("log_levels");

  group.bench_function("trace", |b| b.iter(|| tracing::trace!("Trace message")));

  group.bench_function("debug", |b| b.iter(|| tracing::debug!("Debug message")));

  group.bench_function("info", |b| b.iter(|| tracing::info!("Info message")));

  group.bench_function("warn", |b| b.iter(|| tracing::warn!("Warning message")));

  group.bench_function("error", |b| b.iter(|| tracing::error!("Error message")));

  group.finish();
}

fn bench_backpressure(c: &mut Criterion) {
  // Test what happens when writer can't keep up
  let trace_system = Trace::init(64, 8); // Small buffers to trigger backpressure

  c.bench_function("backpressure_scenario", |b| {
    b.iter(|| {
      for i in 0..1000 {
        tracing::info!("Backpressure test {}", i);
      }
    })
  });
}

fn bench_snapshot_creation(c: &mut Criterion) {
  let trace_system = Trace::init(1024, 128);

  // Fill up the buffer first
  for i in 0..500 {
    tracing::info!("Setup message {}", i);
  }

  // Small delay to let events get processed
  thread::sleep(Duration::from_millis(10));

  c.bench_function("snapshot_request", |b| {
    b.iter(|| {
      trace_system.request_snapshot("benchmark");
    })
  });
}

criterion_group!(
  benches,
  bench_single_threaded_logging,
  bench_multi_threaded_logging,
  bench_structured_logging,
  bench_log_levels,
  bench_backpressure,
  bench_snapshot_creation
);

criterion_main!(benches);
