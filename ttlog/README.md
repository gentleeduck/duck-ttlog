<p align="center">
  <img src="../public/logo-dark.svg" alt="ttlog" width="120"/>
</p>

<h1 align="center">ttlog</h1>

<p align="center">
  Lock-free structured logging core. Ring buffers, thread-local string interning, compressed crash snapshots.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/ttlog">crates.io</a> -
  <a href="https://docs.rs/ttlog">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/ttlog"><img src="https://img.shields.io/crates/v/ttlog.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/ttlog"><img src="https://docs.rs/ttlog/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/ttlog.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add ttlog
```

## Quick start

```rust
use ttlog::Logger;

let log = Logger::new();
log.info("startup", &[("port", "8080")]);
```

## Docs

- [crates.io](https://crates.io/crates/ttlog)
- [docs.rs](https://docs.rs/ttlog)
- Per-crate guide in the repo: see [`../README.md`](../README.md)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
