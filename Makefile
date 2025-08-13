# Makefile

.PHONY: all build test check format release

all: check format test build

build:
	cargo build

test:
	cargo test

check:
	cargo check

format:
	cargo +nightly fmt --all -- --config-path ./rustfmt.toml


release:
	cargo build --release

