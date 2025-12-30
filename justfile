# tael justfile

# Default recipe - show available commands
default:
    @just --list

# Run all CI checks locally
ci: fmt-check clippy test build

# Run tests
test:
    cargo test

# Run tests with output
test-verbose:
    cargo test -- --nocapture

# Run clippy lints
clippy:
    cargo clippy --all-targets -- -D warnings

# Check formatting
fmt-check:
    cargo fmt --check

# Format code
fmt:
    cargo fmt

# Build debug
build:
    cargo build

# Build release
release:
    cargo build --release

# Run the TUI
run *ARGS:
    cargo run -- {{ARGS}}

# Clean build artifacts
clean:
    cargo clean

# Watch for changes and run tests
watch:
    cargo watch -x test

# Install locally
install:
    cargo install --path .
