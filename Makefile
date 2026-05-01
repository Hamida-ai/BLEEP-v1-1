.PHONY: all build test clean fmt clippy check help

# Default target
all: check test

# Build the project
build:
	cargo build --release

# Run tests
test:
	cargo test --lib

# Clean build artifacts
clean:
	cargo clean

# Format code
fmt:
	cargo fmt --all --check

# Run clippy
clippy:
	cargo clippy --all-targets -- -D warnings

# Check compilation
check:
	cargo check

# Full CI check
ci: fmt clippy check test

# Help
help:
	@echo "Available targets:"
	@echo "  build    - Build the project in release mode"
	@echo "  test     - Run library tests"
	@echo "  clean    - Clean build artifacts"
	@echo "  fmt      - Format code with rustfmt"
	@echo "  clippy   - Run clippy lints"
	@echo "  check    - Check compilation without building"
	@echo "  ci       - Run full CI checks (fmt, clippy, check, test)"
	@echo "  help     - Show this help message"
