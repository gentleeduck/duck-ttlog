# Changelog

All notable changes to this project will be documented in this file.

The format is based on [Keep a Changelog](https://keepachangelog.com/en/1.1.0/),
and this project adheres to
[Semantic Versioning](https://semver.org/spec/v2.0.0.html).

## [Unreleased]

## [0.1.21](https://github.com/gentleeduck/duck-ttlog/releases/tag/ttlog-benches-v0.1.21) - 2026-05-05

### Added

- *(examples)* add distributed example and update snapshot/event handling
- *(readme)* ignore
- *(trace)* introduce distributed tracing support with trace_id
- *(snapshot)* ensure storage directory exists before writing snapshot

### Other

- 0.1.21 + per-crate READMEs with shared logo
- sync ci scaffolding from duck-mc + relax clippy
- Add latest benchmark audit report snapshot
- Fix dynamic KV logging and harden benchmark/reporting reliability
- Enhance logging capabilities and introduce new event builder
- ignore
- *(readme)* ignore
- improve listeners, snapshot, and string interner + add benchmark results
- *(benchmarks)* still working on it
- optimize max_performance bench and improve snapshot/string_interner
- *(core)* remove event_builder, update examples and core modules
- Improve logging performance and refactor internals
- *(event)* update logging internals and cleanup benches
- *(core)* remove old logger module and update event/trace handling
- update Cargo.toml and max_performance.rs for performance tuning
- 🔥 Performance & Bench Enhancements: Add max throughput test and optimize internals
- update and add new benchmark reports and improvements
- Add StringInterner module and update benchmarks
- *(benches)* move benchmarks into separate ttlog-benches crate
