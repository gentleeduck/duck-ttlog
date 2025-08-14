# Makefile Usage Guide

This project includes a comprehensive Makefile that automates common development tasks.

## Quick Start

```bash
# Show all available commands
make help

# Run the complete development workflow
make all

# Install required development tools
make install-tools
```

## Core Commands

### Development Workflow
- `make all` - Run check, format, lint, and test (recommended before commits)
- `make dev` - Quick development checks (check, format, lint)

### Building
- `make build` - Build all crates
- `make release` - Build release versions
- `make clean` - Clean all build artifacts

### Testing
- `make test` - Run all tests
- `make test-ttlog` - Test only the main ttlog crate
- `make test-ttlog-view` - Test only the ttlog-view crate
- `make test-examples` - Test all example crates

### Code Quality
- `make check` - Check all crates compile
- `make format` - Format all code with rustfmt
- `make lint` - Run clippy linting
- `make check-tests` - Verify all tests are in `__test__` modules

### Documentation
- `make docs` - Generate and open documentation
- `make docs-serve` - Generate documentation and serve locally

## Individual Crate Commands

You can also work with specific crates:

```bash
# Build specific crates
make build-ttlog
make build-ttlog-view

# Check specific crates
make check-ttlog
make check-ttlog-view

# Format specific crates
make format-ttlog
make format-ttlog-view
```

## Advanced Commands

### Code Coverage
```bash
make coverage  # Requires cargo-tarpaulin
```

### Security & Dependencies
```bash
make audit     # Check for security vulnerabilities
make update    # Update dependencies
make outdated  # Check for outdated dependencies
```

### Benchmarks
```bash
make bench     # Run benchmarks (if available)
```

## Prerequisites

Make sure you have the following tools installed:

```bash
# Install Rust toolchain
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Install development tools
make install-tools
```

## Test Organization

All tests in this project are properly organized in `__test__` modules within each source directory. The `make check-tests` command verifies this organization.

## Workspace Structure

This project uses Cargo workspaces to manage multiple crates:
- `ttlog` - Main library crate
- `ttlog-view` - Viewer application
- `examples/*` - Example applications

## Troubleshooting

### Common Issues

1. **Nightly toolchain required for formatting**
   ```bash
   rustup toolchain install nightly
   rustup component add rustfmt --toolchain nightly
   ```

2. **Clippy not found**
   ```bash
   rustup component add clippy
   ```

3. **Workspace commands fail**
   - Ensure you're in the root directory
   - Check that `Cargo.toml` workspace is properly configured

### Getting Help

```bash
make help  # Show all available commands
```

## Contributing

When contributing to this project:

1. Run `make all` before submitting PRs
2. Ensure all tests pass with `make test`
3. Format code with `make format`
4. Check for linting issues with `make lint`
5. Verify test organization with `make check-tests` 