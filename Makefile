# Comprehensive Makefile for ttlog project
# Provides commands for development, testing, formatting, and documentation

.PHONY: help all build test check format lint clean release docs install-tools check-tests

# Default target
help:
	@echo "Available commands:"
	@echo "  help          - Show this help message"
	@echo "  all           - Run check, format, lint, and test"
	@echo "  build         - Build all crates"
	@echo "  test          - Run all tests"
	@echo "  check         - Check all crates compile"
	@echo "  format        - Format all code with rustfmt"
	@echo "  lint          - Run clippy linting"
	@echo "  clean         - Clean all build artifacts"
	@echo "  release       - Build release versions"
	@echo "  docs          - Generate and open documentation"
	@echo "  docs-serve    - Serve documentation locally"
	@echo "  install-tools - Install required development tools"
	@echo "  check-tests   - Verify all tests are in __test__ modules"

# Main development workflow
all: check format lint test

# Build all crates
build:
	@echo "Building all crates..."
	cargo build --workspace

# Run all tests
test:
	@echo "Running all tests..."
	cargo test --workspace --verbose

# Check all crates compile
check:
	@echo "Checking all crates..."
	cargo check --workspace

# Format all code
format:
	@echo "Formatting code..."
	cargo +nightly fmt --all -- --config-path ./rustfmt.toml

# Run clippy linting
lint:
	@echo "Running clippy..."
	cargo clippy --workspace -- -D warnings

# Clean build artifacts
clean:
	@echo "Cleaning build artifacts..."
	cargo clean --workspace

# Build release versions
release:
	@echo "Building release versions..."
	cargo build --release --workspace

# Generate documentation
docs:
	@echo "Generating documentation..."
	cargo doc --workspace --no-deps --open

# Serve documentation locally
docs-serve:
	@echo "Serving documentation locally..."
	cargo doc --workspace --no-deps
	@echo "Documentation available at: target/doc/ttlog/index.html"
	@echo "Run 'python3 -m http.server 8000' in target/doc/ to serve"

# Install required development tools
install-tools:
	@echo "Installing development tools..."
	rustup component add rustfmt --toolchain nightly
	rustup component add clippy
	@echo "Tools installed successfully!"

# Verify all tests are in __test__ modules
check-tests:
	@echo "Checking test organization..."
	@echo "Looking for tests outside of __test__ modules..."
	@if find . -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | grep -v "__test__" | grep -v "examples"; then \
		echo "WARNING: Found tests outside of __test__ modules:"; \
		find . -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | grep -v "__test__" | grep -v "examples"; \
		echo "Consider moving these to __test__ modules."; \
	else \
		echo "✓ All tests are properly organized in __test__ modules"; \
	fi

# Quick development check
dev: check format lint
	@echo "Development checks completed successfully!"

# Run tests with coverage (requires cargo-tarpaulin)
coverage:
	@echo "Running tests with coverage..."
	cargo tarpaulin --workspace --out Html --output-dir coverage

# Run benchmarks (if available)
bench:
	@echo "Running benchmarks..."
	cargo bench --workspace

# Check for security vulnerabilities
audit:
	@echo "Checking for security vulnerabilities..."
	cargo audit

# Update dependencies
update:
	@echo "Updating dependencies..."
	cargo update --workspace

# Check outdated dependencies
outdated:
	@echo "Checking for outdated dependencies..."
	cargo outdated --workspace

# Run specific crate tests
test-ttlog:
	@echo "Testing ttlog crate..."
	cd ttlog && cargo test

test-ttlog-view:
	@echo "Testing ttlog-view crate..."
	cd ttlog-view && cargo test

test-examples:
	@echo "Testing example crates..."
	cd examples/ttlog-simple && cargo test
	cd examples/ttlog-server && cargo test
	cd examples/ttlog-complex && cargo test
	cd examples/ttlog-filereader && cargo test

# Format specific crates
format-ttlog:
	@echo "Formatting ttlog crate..."
	cd ttlog && cargo +nightly fmt -- --config-path ../rustfmt.toml

format-ttlog-view:
	@echo "Formatting ttlog-view crate..."
	cd ttlog-view && cargo +nightly fmt -- --config-path ../rustfmt.toml

# Build specific crates
build-ttlog:
	@echo "Building ttlog crate..."
	cd ttlog && cargo build

build-ttlog-view:
	@echo "Building ttlog-view crate..."
	cd ttlog-view && cargo build

# Check specific crates
check-ttlog:
	@echo "Checking ttlog crate..."
	cd ttlog && cargo check

check-ttlog-view:
	@echo "Checking ttlog-view crate..."
	cd ttlog-view && cargo check

