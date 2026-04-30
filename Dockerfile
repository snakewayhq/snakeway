# syntax=docker/dockerfile:1
#
# Multi-stage build — produces a minimal distroless image with a statically
# linked MUSL binary.  The final image has no shell, no package manager, and
# runs as a non-root user.
#
# Build:
#   docker build -t snakeway:dev .
#
# Run:
#   docker run --rm -p 80:80 -v /etc/snakeway:/etc/snakeway snakeway:dev

# ---------------------------------------------------------------------------
# Stage 1: build
# ---------------------------------------------------------------------------
FROM rust:1.82-bookworm AS builder

RUN apt-get update && apt-get install -y --no-install-recommends \
        musl-tools \
    && rm -rf /var/lib/apt/lists/*

RUN rustup target add x86_64-unknown-linux-musl

WORKDIR /build
COPY . .

# Use the `dist` profile: optimized, stripped, no debug symbols.
RUN cargo build --profile dist --target x86_64-unknown-linux-musl -p snakeway

# ---------------------------------------------------------------------------
# Stage 2: runtime (distroless — no shell, no package manager)
# ---------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=builder /build/target/x86_64-unknown-linux-musl/dist/snakeway \
                    /usr/local/bin/snakeway

# Ship the default config skeleton so the image is runnable out of the box.
COPY packaging/config/ /etc/snakeway/

EXPOSE 80 443

ENV SNAKEWAY_CONFIG=/etc/snakeway

ENTRYPOINT ["/usr/local/bin/snakeway"]
CMD ["run"]
