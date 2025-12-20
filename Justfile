#!/usr/bin/env just --justfile

# Default recipe
default: build

# -----------------------------------------------------------------------------
# ENV VARS
# -----------------------------------------------------------------------------
# The config Snakeway should load when running locally

CONFIG := "config/snakeway.toml"

# -----------------------------------------------------------------------------
# Tools and docs
# -----------------------------------------------------------------------------

install-tools:
    cargo install cargo-component cargo-zigbuild wit-bindgen-cli samply

docs:
    cd docs && npm run docs:dev

# -----------------------------------------------------------------------------
# Benchmarks and profiling
# -----------------------------------------------------------------------------

# Run hey to test out various static file request configs.
benchmark-static-files:
    @echo "No compression..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/static/index.html
    @echo "Gzip..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/static/1kb.html
    @echo "Brotli..."
    hey -n 20000 -c 128 -H "Accept-Encoding: br" http://127.0.0.1:8080/static/6kb.html
    @echo "Range request..."
    hey -n 5000 -c 128 -H "Range: bytes=0-99" http://127.0.0.1:8080/static/images/1mb.png
    @echo "Head request..."
    hey -n 20000 -c 128 -m HEAD -H "Accept-Encoding: gzip" http://127.0.0.1:8080/static/index.html

# Start this profile recipe, then run run-load. When this command exits, the results should be displayed.
profile:
    @echo "Building Snakeway (release, with symbols)"
    cargo build --release -p snakeway --features static_files
    @echo "Profiling with Samply (Ctrl+C to stop)"
    samply record target/release/snakeway > /dev/null

# Generate meaningful profiling data against an upstream.
run-load-against-upstream:
    hey -n 300000 -c 256 http://127.0.0.1:8080/__metrics

# Generate meaningful profiling data against a static file.
run-load-against-static:
    hey -n 300000 -c 256 http://127.0.0.1:8080/static/index.html

# Debugging
# -----------------------------------------------------------------------------

# Show max vs used file descriptors
debug-file-descriptors:
    @echo "Max file descriptors:"
    @ulimit -n
    @echo "\nCurrent descriptors in use:"
    @lsof -p $(pgrep snakeway) | wc -l

# -----------------------------------------------------------------------------
# BUILD TASKS
# -----------------------------------------------------------------------------

# Create WIT bindings for example WASM device
generate-wit-bindings:
    @echo "Generate bindings for WASM devices"
    wit-bindgen rust ./snakeway-wit/wit \
      --world snakeway \
      --out-dir ./snakeway-wit/src/

# Build debug binary
build:
    cargo build

# Build optimized release binary
release:
    cargo build --release

# Install build tools
install-build-tools:
    @brew install zig
    @cargo install cargo-zigbuild

# MUSL Build: AARCH64 (Linux ARM64)
musl-aarch64:
    rustup target add aarch64-unknown-linux-musl
    cargo zigbuild --release --target aarch64-unknown-linux-musl

# MUSL Build: X86_64 (Linux Intel)
musl-x86_64:
    rustup target add x86_64-unknown-linux-musl
    cargo zigbuild --release --target x86_64-unknown-linux-musl

# Build both
musl-all: musl-aarch64 musl-x86_64

# Run Snakeway locally
run:
    cargo run -- --config {{ CONFIG }}

# Build and run
build-and-run: release
    ./target/release/snakeway --config {{ CONFIG }}

# -----------------------------------------------------------------------------
# LINTING & FORMAT
# -----------------------------------------------------------------------------

fmt:
    cargo fmt

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

check:
    cargo check

lint: fmt clippy

# -----------------------------------------------------------------------------
# TESTS
# -----------------------------------------------------------------------------

test:
    cargo test

integration-test:
    cargo test -p integration-tests

# -----------------------------------------------------------------------------
# CLEANUP
# -----------------------------------------------------------------------------

clean:
    cargo clean

# -----------------------------------------------------------------------------
# BUNDLE A LOCAL DEVELOPER RELEASE
# -----------------------------------------------------------------------------
# This creates `dist/` with:
# - x86_64 musl binary
# - aarch64 musl binary
# - example configs

bundle:
    # Create directories
    rm -rf dist
    mkdir -p dist/bin
    mkdir -p dist/config

    # Build musl binaries
    just musl-all

    # Copy binaries
    cp target/x86_64-unknown-linux-musl/release/snakeway dist/bin/snakeway-x86_64
    cp target/aarch64-unknown-linux-musl/release/snakeway dist/bin/snakeway-aarch64

    # Copy configs
    cp config/snakeway.toml.example dist/config/

    echo "Bundle created in dist/"
