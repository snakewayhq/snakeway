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
      --file Dockerfile.ci \
      --platform linux/amd64,linux/arm64 \
      --tag snakeway:dev \
      .
