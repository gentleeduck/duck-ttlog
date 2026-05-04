# Contributing to dmc

Thanks for the interest. This file covers the workflow + style. For
deeper architecture or per-crate guidance see
[`dmc-docs/guides/contributing.md`](dmc-docs/guides/contributing.md).

## Repo layout

```
duck-mc/
|- dmc-lexer/       crate
|- dmc-parser/      crate
|- dmc-highlight/   crate (leaf, syntect bundle)
|- dmc-transform/   crate
|- dmc-codegen/     crate
|- dmc-diagnostic/  crate
|- dmc-schema/      crate
|- dmc-core/        crate (engine + CLI)
|- dmc-napi/        crate (cdylib + TS wrapper, @gentleduck/md)
|- dmc-sidecar/     pure JS (Node helper)
|- dmc-docs/        per-crate + architecture docs
|- duck-benchmarks/ recorded bench runs per phase
|- examples/        nextjs, nextjs-velite, acme-docs, web
`- docs/            architecture + benchmark write-ups
```

## Build

```sh
pnpm install
cargo build --release -p dmc-core --features pretty-code
cargo test  --workspace --features pretty-code
pnpm --filter @gentleduck/md run build   # napi binary
```

## Pre-commit

Husky runs `cargo fmt apply` automatically. Before pushing:

```sh
cargo fmt
cargo clippy --workspace --all-features -- -D warnings
cargo test  --workspace --features pretty-code
```

## Style

- ASCII-only in source + prose. Use `-`, `'`, `"`, `...`, `->`, `<-`,
  `>=`, `<=`, `!=`, `*`, `.`. No em-dashes, curly quotes, ellipsis,
  arrows, comparison glyphs in unicode form.
- Caveman-mode terse comments. No filler. Comments explain WHY, not
  WHAT.
- Conventional commit subjects: `kind(scope): subject`. Examples:
  `fix(parser): support lists nested inside blockquotes`,
  `feat(highlight): bundled syntect grammars`.

## Tests

Per-crate `tests/*.rs` for integration; `#[cfg(test)] mod tests` for
unit. Snapshot tests via `insta` in `dmc-core` cover compile output
fixtures.

## Adding a transformer / theme / grammar / diagnostic code

See [`dmc-docs/guides/contributing.md`](dmc-docs/guides/contributing.md)
for the per-extension checklist (file paths, feature gates, doc
locations).

## PR checklist

- [ ] Tests pass: `cargo test --workspace --features pretty-code`
- [ ] Clippy clean: `cargo clippy --workspace --all-features -- -D warnings`
- [ ] Docs updated (per-crate + cheatsheet if API changes)
- [ ] No special chars in prose (em-dash, curly quotes, etc)
- [ ] Bench numbers if perf-relevant change
  (see [`duck-benchmarks/`](duck-benchmarks/))
- [ ] Cache key updated if compile output shape changes

## Reporting bugs / requesting features

Open an issue at
[github.com/gentleeduck/duck-mc/issues](https://github.com/gentleeduck/duck-mc/issues).
For security issues, see [`SECURITY.md`](SECURITY.md).

## License

Contributions are licensed under MIT (see [`LICENSE`](LICENSE)) by
default.
