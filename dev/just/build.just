# Create WIT bindings for example WASM device
generate-wit-bindings:
    @echo "Generate bindings for WASM devices"
    wit-bindgen rust ./crates/snakeway-wit/wit \
      --world snakeway \
      --out-dir ./crates/snakeway-wit/src/

# Install packaging tools (cargo-deb, cargo-generate-rpm)
install-package-tools:
    cargo install cargo-deb cargo-generate-rpm --locked

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

# Use act to run the CI "validate" build job locally
run-gh-validate:
    mkdir -p data/sccache/ data/artifacts/
    act -j validate \
      --privileged \
      -v $(pwd)/data/sccache:/root/.cache/sccache \
      --env SCCACHE_DIR=/root/.cache/sccache \
      --artifact-server-path $(pwd)/data/artifacts
