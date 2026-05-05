<p align="center">
  <img src="../public/logo-dark.svg" alt="ttlog-macros" width="120"/>
</p>

<h1 align="center">ttlog-macros</h1>

<p align="center">
  Macro helpers for ttlog: `#[trace_fn]`, `log!`.
</p>

<p align="center">
  <a href="../LICENSE">MIT</a> -
  <a href="../CHANGELOG.md">Changelog</a> -
  <a href="../CONTRIBUTING.md">Contributing</a> -
  <a href="https://crates.io/crates/ttlog-macros">crates.io</a> -
  <a href="https://docs.rs/ttlog-macros">docs.rs</a>
</p>

<p align="center">
  <a href="https://crates.io/crates/ttlog-macros"><img src="https://img.shields.io/crates/v/ttlog-macros.svg" alt="crates.io"/></a>
  <a href="https://docs.rs/ttlog-macros"><img src="https://docs.rs/ttlog-macros/badge.svg" alt="docs.rs"/></a>
  <a href="../LICENSE"><img src="https://img.shields.io/crates/l/ttlog-macros.svg" alt="MIT"/></a>
</p>

---

## Install

```sh
cargo add ttlog-macros
```

## Quick start

```rust
use ttlog_macros::trace_fn;

#[trace_fn]
fn handle(req: Request) -> Response { /* */ }
```

## Docs

- [crates.io](https://crates.io/crates/ttlog-macros)
- [docs.rs](https://docs.rs/ttlog-macros)
- Per-crate guide in the repo: see [`../README.md`](../README.md)

## Contributing

See [`../CONTRIBUTING.md`](../CONTRIBUTING.md).

## License

MIT. See [`../LICENSE`](../LICENSE).
