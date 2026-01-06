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

install-tools: install-dev-tools install-build-tools
    @echo "Finished installing tools"

# If on a Mac, install packages with homebrew.
brew-install packages:
    {{ if os() == "macos" { "brew install " + packages } else { "" } }}

install-dev-tools:
    go install google.golang.org/protobuf/cmd/protoc-gen-go@latest
    go install google.golang.org/grpc/cmd/protoc-gen-go-grpc@latest
    just brew-install "grpcurl go"
    bun i -g wscat
    cargo install tokio-console samply cargo-nextest

docs:
    cd docs && bun start

# Install mkcert and nss, then create dev certs.
setup-tls-dev-cert:
    just brew-install "mkcert nss"
    mkdir -p ./data/certs/
    mkcert -install
    mkcert \
      -cert-file ./data/certs/snakeway.pem \
      -key-file ./data/certs/snakeway.key \
      localhost 127.0.0.1 ::1

# -----------------------------------------------------------------------------
# Benchmarks and profiling
# -----------------------------------------------------------------------------

# Use wrk and hey to test various upstream configurations.
benchmark-proxy:
    @echo "No TLS (wrk)"
    wrk -t4 -c128 -d10s http://localhost:8080/api/users/1
    @echo "With TLS (wrk)"
    wrk -t4 -c128 -d10s https://localhost:8443/api/users/1
    @echo "Raw upstream (wrk)"
    wrk -t4 -c128 -d10s http://localhost:3000/api/users/1
    @echo "To TLS (hey)"
    hey -n 20000 -c 128 http://127.0.0.1:8080/api/users/1
    @echo "Raw upstream (hey)"
    hey -n 20000 -c 128 http://127.0.0.1:3000/api/users/1

# Run hey to test out various static file request configs.
benchmark-static-files:
    @echo "No compression..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/
    @echo "Gzip..."
    hey -n 20000 -c 128 -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/1kb.html
    @echo "Brotli..."
    hey -n 20000 -c 128 -H "Accept-Encoding: br" http://127.0.0.1:8080/assets/6kb.html
    @echo "Range request..."
    hey -n 5000 -c 128 -H "Range: bytes=0-99" http://127.0.0.1:8080/assets/images/1mb.png
    @echo "Head request..."
    hey -n 20000 -c 128 -m HEAD -H "Accept-Encoding: gzip" http://127.0.0.1:8080/assets/index.html

# Start this profile recipe, then run run-load. When this command exits, the results should be displayed.
profile:
    @echo "Building Snakeway (release, with symbols)"
    cargo build --release -p snakeway --features static_files
    @echo "Profiling with Samply (Ctrl+C to stop)"
    samply record target/release/snakeway > /dev/null

# Start this profile recipe, then run "tokio-console" at the cli
profile-tokio:
    RUSTFLAGS="--cfg tokio_unstable" TOKIO_CONSOLE=1 RUST_LOG=error cargo run -r -p snakeway --features static_files > /dev/null

# Generate meaningful profiling data against an upstream.
run-load-against-upstream:
    hey -n 300000 -c 256 http://127.0.0.1:8080/api/users/1

# Generate meaningful profiling data against a static file.
run-load-against-static:
    hey -n 300000 -c 256 http://127.0.0.1:8080/assets/index.html

# Generate some spoofed traffic for the identity device
run-spoofed-traffic:
    k6 run --vus 10 --duration 30s spoof-traffic.js

# Build once, then run (faster restarts)
build-origin:
    @echo "Generate proto code..."
    (cd snakeway-origin && ./generate-proto.sh)
    @echo "Starting Go origin server (http, https, ws, wss, grpc)..."
    cd snakeway-origin && go build -o origin-server .

# Build and run the upstream.
start-origin:
    TLS_CERT_FILE=./integration-tests/certs/server.pem TLS_KEY_FILE=./integration-tests/certs/server.key ./snakeway-origin/origin-server

# Check all 5 upstream channels
sanity-check-origin:
    @echo "HTTP:"
    @curl -s http://localhost:3000/

    @echo "\nHTTPS:"
    @curl -s --cacert integration-tests/certs/ca.pem https://localhost:3443/

    @echo "\nWS:"
    @(echo "Hello, websocket." | wscat -c ws://localhost:3000/ws)

    @echo "\nWSS:"
    @(echo "Hello, secure websocket." | NODE_EXTRA_CA_CERTS=integration-tests/certs/ca.pem wscat -c wss://localhost:3443/ws)

    @echo "\ngRPC:"
    @grpcurl \
      -cacert integration-tests/certs/ca.pem \
      -proto snakeway-origin/users.proto \
      -d '{"id":"123"}' \
      localhost:5051 \
      users.UserService/GetUser

# -----------------------------------------------------------------------------
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
    @cargo install cargo-component cargo-zigbuild wit-bindgen-cli

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

# Install Snakeway dev CA into macOS System keychain
install-dev-ca:
    @echo "Installing Snakeway dev CA (macOS system trust)…"
    sudo security add-trusted-cert \
      -d -r trustRoot \
      -k /Library/Keychains/System.keychain \
      integration-tests/certs/ca.pem
    @echo "✓ Snakeway dev CA installed"

# Remove Snakeway dev CA from macOS System keychain
uninstall-dev-ca:
    @echo "Removing Snakeway dev CA from macOS system trust…"
    sudo security delete-certificate \
      -c "Snakeway Dev Root CA (DO NOT TRUST IN PROD)" \
      /Library/Keychains/System.keychain
    @echo "✓ Snakeway dev CA removed"

generate-dev-certs:
    [ ! -d "integration-tests/certs" ] && ./gen-test-certs.sh || true

test:
    cargo nextest run -p snakeway-core --features static_files,wasm

integration-test: generate-dev-certs
    cargo nextest run -p integration-tests

test-everything: lint test integration-test
    @echo "All good."

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
