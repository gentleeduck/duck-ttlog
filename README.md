<p align="center">
  <img src="./public/logo-dark.svg" alt="ttlog" width="120"/>
</p>

<h1 align="center">ttlog</h1>

<p align="center">
  Lock-free structured logging for Rust. Ring buffers, thread-local string
  interning, compressed crash snapshots.
</p>

<p align="center">
  <a href="./LICENSE">MIT</a> -
  <a href="https://crates.io/crates/ttlog">crates.io</a> -
  <a href="https://docs.rs/ttlog">docs.rs</a> -
  <a href="./ttlog-benches">benches</a> -
  <a href="https://github.com/gentleeduck/duck-ttlog/issues">issues</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/ttlog"><img src="https://img.shields.io/crates/v/ttlog.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/ttlog"><img src="https://docs.rs/ttlog/badge.svg" alt="docs.rs"/></a>
  <a href="./LICENSE"><img src="https://img.shields.io/crates/l/ttlog.svg" alt="MIT"/></a>
</p>

---

## Headline

| metric | value |
| --- | ---: |
| throughput | 318M events/sec @ 16 threads |
| buffer ops | 15M ops/sec (producer-heavy) |
| memory | 24 bytes per event |
| concurrency | 256+ threads tested |

Numbers from `ttlog-benches`. Re-run on your hardware before quoting.

## Install

```sh
cargo add ttlog
```

## Quick start

```rust
use ttlog::{Logger, LogLevel};

let log = Logger::new();
log.info("startup", &[("port", "8080"), ("env", "prod")]);
log.warn("slow query", &[("ms", "850")]);
log.error("upstream 503", &[("path", "/api/users")]);
```

Drop-in replacement for `tracing` or `log` in hot paths where
allocation matters. Compressed snapshots on panic so you can replay.

## Workspace

| crate | role |
| --- | --- |
| [`ttlog`](ttlog) | core logger + ring buffers |
| [`ttlog-macros`](ttlog-macros) | `#[trace_fn]`, `log!` macros |
| [`ttlog-view`](ttlog-view) | snapshot reader / pretty printer |
| [`ttlog-benches`](ttlog-benches) | criterion suite |
| [`examples/`](examples) | runnable demos: simple, server, complex, filereader |

## Build

```sh
cargo build --release --workspace
cargo test  --workspace
cargo bench -p ttlog-benches
```

## Crash recovery

Each thread holds a lock-free ring buffer. On panic, a SIGSEGV
handler dumps every live buffer to a compressed snapshot. Replay
with `ttlog-view`:

```sh
ttlog-view snapshot.ttlog
```

## Performance properties

- Lock-free SPSC ring per thread; lockstep across cores.
- Thread-local string interner caps per-message allocations at zero
  for repeated keys.
- Compressed snapshot on panic via zstd.
- No global mutex; no async runtime; no allocator hooks.

Detailed write-up + flame graphs under [`docs/`](docs).

## Contributing

PR checklist + style notes in [`CONTRIBUTING.md`](CONTRIBUTING.md).
Security: [`SECURITY.md`](SECURITY.md).
Behaviour: [`CODE_OF_CONDUCT.md`](CODE_OF_CONDUCT.md).

## License

MIT. See [`LICENSE`](LICENSE).
