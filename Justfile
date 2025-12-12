#!/usr/bin/env just --justfile

# Default recipe
default: build

# -----------------------------------------------------------------------------
# ENV VARS
# -----------------------------------------------------------------------------
# The config Snakeway should load when running locally

CONFIG := "config/snakeway.toml"


# -----------------------------------------------------------------------------
# Tools
# -----------------------------------------------------------------------------

install-tools:
    cargo install cargo-component cargo-zigbuild wit-bindgen-cli


# -----------------------------------------------------------------------------
# BUILD TASKS
# -----------------------------------------------------------------------------

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
