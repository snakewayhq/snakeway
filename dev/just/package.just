# Build a .deb package (requires cargo-deb — run `just install-package-tools`)
deb:
    cargo deb -p snakeway

# Build a .rpm package (requires cargo-generate-rpm — run `just install-package-tools`)
rpm:
    cargo generate-rpm -p snakeway

# Build the Docker image locally (from source, native arch)
docker-build:
    docker build -t snakeway:dev .

# Build the Docker image using pre-built binaries (CI pattern, multi-arch)

# Expects binaries staged at dist/linux/amd64/snakeway and dist/linux/arm64/snakeway
docker-build-ci:
    docker buildx build \
      --file packaging/docker/Dockerfile \
      --platform linux/amd64,linux/arm64 \
      --tag snakeway:dev \
      .

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
