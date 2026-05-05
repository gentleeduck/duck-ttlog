<p align="center">
  <img src="../../public/logo-dark.svg" alt="ttlog-simple" width="120"/>
</p>

<h1 align="center">ttlog-simple</h1>

<p align="center">
  Minimal ttlog setup demonstrating basic events, levels, and crash snapshot.
</p>

<p align="center">
  <a href="../../LICENSE">MIT</a> -
  <a href="../../README.md">repo</a>
</p>

---

## Run

```sh
cargo run -p ttlog-simple
```

## Modules

- `example_simple.rs`           plain log calls
- `example_basic_logging.rs`    levels + targets
- `example_structured_logging.rs` key-value pairs
- `example_multithreaded_logging.rs` shared logger across threads
- `example_high_volume_logging.rs` throughput stress
- `example_panic_handling.rs`   crash snapshot via signal_hook
- `example_error_scenarios.rs`  recoverable + fatal errors
- `example_custom_service.rs`   custom listener
- `example_distributed.rs`      multi-process log fanout
