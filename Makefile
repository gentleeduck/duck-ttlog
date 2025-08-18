# Comprehensive Makefile for TTLog project
# Provides commands for development, testing, formatting, and documentation

.PHONY: help all build test check format lint clean release docs install-tools check-tests bench benchmark-report

# Default target
help:
	@echo "🚀 TTLog - High-Performance Distributed Logging Library"
	@echo "=================================================="
	@echo ""
	@echo "📦 Core Commands:"
	@echo "  help          - Show this help message"
	@echo "  all           - Run check, format, lint, and test"
	@echo "  build         - Build all crates"
	@echo "  test          - Run all tests"
	@echo "  check         - Check all crates compile"
	@echo "  format        - Format all code with rustfmt"
	@echo "  lint          - Run clippy linting"
	@echo "  clean         - Clean all build artifacts"
	@echo "  release       - Build release versions"
	@echo ""
	@echo "📊 Benchmarking:"
	@echo "  bench         - Run all benchmarks"
	@echo "  bench-quick   - Run quick benchmarks"
	@echo "  bench-distributed - Run distributed system benchmarks"
	@echo "  bench-stress  - Run stress tests"
	@echo "  benchmark-report - Generate comprehensive benchmark report"
	@echo ""
	@echo "📚 Documentation:"
	@echo "  docs          - Generate and open documentation"
	@echo "  docs-serve    - Serve documentation locally"
	@echo "  docs-build    - Build documentation without opening"
	@echo ""
	@echo "🛠️ Development:"
	@echo "  install-tools - Install required development tools"
	@echo "  check-tests   - Verify all tests are in __test__ modules"
	@echo "  dev           - Quick development checks"
	@echo "  coverage      - Run tests with coverage"
	@echo ""
	@echo "🔍 Analysis:"
	@echo "  audit         - Check for security vulnerabilities"
	@echo "  update        - Update dependencies"
	@echo "  outdated      - Check for outdated dependencies"
	@echo ""
	@echo "📁 Individual Crates:"
	@echo "  build-ttlog   - Build main ttlog crate"
	@echo "  build-view    - Build ttlog-view crate"
	@echo "  build-event   - Build ttlog-event crate"
	@echo "  test-ttlog    - Test main ttlog crate"
	@echo "  test-view     - Test ttlog-view crate"
	@echo "  test-event    - Test ttlog-event crate"
	@echo "  test-examples - Test all example crates"
	@echo ""
	@echo "🎯 Examples:"
	@echo "  run-simple    - Run simple example"
	@echo "  run-server    - Run server example"
	@echo "  run-complex   - Run complex example"
	@echo "  run-filereader - Run file reader example"

# Main development workflow
all: check format lint test

# Build all crates
build:
	@echo "🔨 Building all crates..."
	cargo build --workspace

# Run all tests
test:
	@echo "🧪 Running all tests..."
	cargo test --workspace --verbose

# Check all crates compile
check:
	@echo "✅ Checking all crates..."
	cargo check --workspace

# Format all code
format:
	@echo "🎨 Formatting code..."
	cargo +nightly fmt --all -- --config-path ./rustfmt.toml

# Run clippy linting
lint:
	@echo "🔍 Running clippy..."
	cargo clippy --workspace -- -D warnings

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean --workspace

# Build release versions
release:
	@echo "🚀 Building release versions..."
	cargo build --release --workspace

# Generate documentation
docs:
	@echo "📚 Generating documentation..."
	cargo doc --workspace --no-deps --open

# Build documentation without opening
docs-build:
	@echo "📚 Building documentation..."
	cargo doc --workspace --no-deps

# Serve documentation locally
docs-serve:
	@echo "🌐 Serving documentation locally..."
	cargo doc --workspace --no-deps
	@echo "Documentation available at: target/doc/ttlog/index.html"
	@echo "Run 'python3 -m http.server 8000' in target/doc/ to serve"

# Install required development tools
install-tools:
	@echo "🛠️ Installing development tools..."
	rustup component add rustfmt --toolchain nightly
	rustup component add clippy
	@echo "✅ Tools installed successfully!"

# Verify all tests are in __test__ modules
check-tests:
	@echo "🔍 Checking test organization..."
	@echo "Looking for tests outside of __test__ modules..."
	@if find . -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | grep -v "__test__" | grep -v "examples"; then \
		echo "⚠️  WARNING: Found tests outside of __test__ modules:"; \
		find . -name "*.rs" -exec grep -l "#\[cfg(test)\]" {} \; | grep -v "__test__" | grep -v "examples"; \
		echo "Consider moving these to __test__ modules."; \
	else \
		echo "✅ All tests are properly organized in __test__ modules"; \
	fi

# Quick development check
dev: check format lint
	@echo "✅ Development checks completed successfully!"

# Run tests with coverage (requires cargo-tarpaulin)
coverage:
	@echo "📊 Running tests with coverage..."
	cargo tarpaulin --workspace --out Html --output-dir coverage

# Run benchmarks
bench:
	@echo "⚡ Running comprehensive benchmarks (ttlog-benches, bench profile)..."
	cd ttlog-benches && cargo bench

# Run quick benchmarks
bench-quick:
	@echo "⚡ Running quick benchmarks (ttlog-benches, bench profile)..."
	cd ttlog-benches && CRITERION_SAMPLE_SIZE=10 CRITERION_MEASUREMENT_TIME=2000 cargo bench

# Run distributed system benchmarks
bench-distributed:
	@echo "🌐 Running distributed system benchmarks (ttlog-benches, bench profile)..."
	cd ttlog-benches && cargo bench --bench distributed_bench

# Run stress tests
bench-stress:
	@echo "🔥 Running stress tests (ttlog-benches, release)..."
	cd ttlog-benches && cargo run --release --bin heavy_stress_test all
	cd ttlog-benches && cargo run --release --bin max_performance all

# Generate comprehensive benchmark report
benchmark-report:
	@echo "📊 Generating comprehensive benchmark report (ttlog-benches, bench profile)..."
	@mkdir -p benchmark_reports
	@echo "TTLog Benchmark Report - $(shell date)" > benchmark_reports/comprehensive_report.txt
	@echo "================================================" >> benchmark_reports/comprehensive_report.txt
	@echo "" >> benchmark_reports/comprehensive_report.txt
	@echo "System Information:" >> benchmark_reports/comprehensive_report.txt
	@echo "  CPU: $(shell nproc) cores" >> benchmark_reports/comprehensive_report.txt
	@echo "  Memory: $(shell free -h | grep Mem | awk '{print $$2}')" >> benchmark_reports/comprehensive_report.txt
	@echo "  Rust Version: $(shell rustc --version)" >> benchmark_reports/comprehensive_report.txt
	@echo "" >> benchmark_reports/comprehensive_report.txt
	@echo "Running benchmarks..." >> benchmark_reports/comprehensive_report.txt
	@cd ttlog-benches && cargo bench 2>&1 | tee -a ../benchmark_reports/comprehensive_report.txt
	@echo "✅ Benchmark report generated: benchmark_reports/comprehensive_report.txt"

# Check for security vulnerabilities
audit:
	@echo "🔒 Checking for security vulnerabilities..."
	cargo audit

# Update dependencies
update:
	@echo "📦 Updating dependencies..."
	cargo update --workspace

# Check outdated dependencies
outdated:
	@echo "📋 Checking for outdated dependencies..."
	cargo outdated --workspace

# Run specific crate tests
test-ttlog:
	@echo "🧪 Testing ttlog crate..."
	cd ttlog && cargo test

test-view:
	@echo "🧪 Testing ttlog-view crate..."
	cd ttlog-view && cargo test

test-event:
	@echo "🧪 Testing ttlog-event crate..."
	cd ttlog-event && cargo test

test-examples:
	@echo "🧪 Testing example crates..."
	cd examples/ttlog-simple && cargo test
	cd examples/ttlog-server && cargo test
	cd examples/ttlog-complex && cargo test
	cd examples/ttlog-filereader && cargo test

# Format specific crates
format-ttlog:
	@echo "🎨 Formatting ttlog crate..."
	cd ttlog && cargo +nightly fmt -- --config-path ../rustfmt.toml

format-view:
	@echo "🎨 Formatting ttlog-view crate..."
	cd ttlog-view && cargo +nightly fmt -- --config-path ../rustfmt.toml

format-event:
	@echo "🎨 Formatting ttlog-event crate..."
	cd ttlog-event && cargo +nightly fmt -- --config-path ../rustfmt.toml

# Build specific crates
build-ttlog:
	@echo "🔨 Building ttlog crate..."
	cd ttlog && cargo build

build-view:
	@echo "🔨 Building ttlog-view crate..."
	cd ttlog-view && cargo build

build-event:
	@echo "🔨 Building ttlog-event crate..."
	cd ttlog-event && cargo build

# Check specific crates
check-ttlog:
	@echo "✅ Checking ttlog crate..."
	cd ttlog && cargo check

check-view:
	@echo "✅ Checking ttlog-view crate..."
	cd ttlog-view && cargo check

check-event:
	@echo "✅ Checking ttlog-event crate..."
	cd ttlog-event && cargo check

# Run examples
run-simple:
	@echo "🚀 Running simple example..."
	cd examples/ttlog-simple && cargo run

run-server:
	@echo "🚀 Running server example..."
	cd examples/ttlog-server && cargo run

run-complex:
	@echo "🚀 Running complex example..."
	cd examples/ttlog-complex && cargo run

run-filereader:
	@echo "🚀 Running file reader example..."
	cd examples/ttlog-filereader && cargo run

# Performance testing
perf-test:
	@echo "⚡ Running performance tests (ttlog-benches, release)..."
	cd ttlog-benches && cargo run --release --bin test_performance
	cd ttlog-benches && cargo run --release --bin heavy_stress_test memory
	cd ttlog-benches && cargo run --release --bin distributed_simulator database

# Memory profiling
mem-profile:
	@echo "🧠 Running memory profiling (ttlog-benches, release)..."
	@if command -v heaptrack >/dev/null 2>&1; then \
		cd ttlog-benches && heaptrack cargo run --release --bin heavy_stress_test all; \
	else \
		echo "⚠️  heaptrack not found. Install with: sudo apt install heaptrack"; \
	fi

# CPU profiling
cpu-profile:
	@echo "🔥 Running CPU profiling (ttlog-benches, release)..."
	@if command -v cargo-flamegraph >/dev/null 2>&1; then \
		cd ttlog-benches && cargo flamegraph --bin heavy_stress_test -- all; \
	else \
		echo "⚠️  cargo-flamegraph not found. Install with: cargo install flamegraph"; \
	fi

# Export full git history for docs
history:
	@echo "📜 Exporting full git history to docs/full_history.txt..."
	./export_git_full_history.sh > docs/full_history.txt
	@echo "✅ Done. See docs/full_history.txt"

# Clean snapshots
clean-snapshots:
	@echo "🧹 Cleaning snapshot files..."
	@find /tmp -name "ttlog-*.bin" -delete 2>/dev/null || true
	@echo "✅ Snapshot files cleaned"

# Check workspace health
workspace-health:
	@echo "🏥 Checking workspace health..."
	@echo "Checking for unused dependencies..."
	@cargo tree --workspace --format "{p} {f}" | grep -E "^\s*[a-zA-Z]" | sort | uniq -c | sort -nr
	@echo ""
	@echo "Checking for duplicate dependencies..."
	@cargo tree --workspace --format "{p} {f}" | grep -E "^\s*[a-zA-Z]" | awk '{print $$2}' | sort | uniq -d
	@echo "✅ Workspace health check completed"

# Generate changelog
changelog:
	@echo "📝 Generating changelog..."
	@if command -v conventional-changelog >/dev/null 2>&1; then \
		conventional-changelog -p angular -i CHANGELOG.md -s; \
	else \
		echo "⚠️  conventional-changelog not found. Install with: npm install -g conventional-changelog-cli"; \
	fi

# Pre-release checks
pre-release: all bench benchmark-report audit workspace-health
	@echo "✅ Pre-release checks completed successfully!"

# Release build
release-build: clean release
	@echo "🚀 Release build completed!"
	@echo "Release artifacts are in target/release/"

# Install viewer tool
install-viewer:
	@echo "📺 Installing ttlog-viewer..."
	cargo install --path ttlog-view
	@echo "✅ ttlog-viewer installed successfully!"

# Run viewer
view-snapshots:
	@echo "📺 Opening ttlog-viewer..."
	@if command -v ttlog-view >/dev/null 2>&1; then \
		ttlog-view /tmp/ttlog-*.bin; \
	else \
		echo "⚠️  ttlog-view not found. Install with: make install-viewer"; \
	fi

