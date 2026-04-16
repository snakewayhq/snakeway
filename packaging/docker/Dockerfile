# syntax=docker/dockerfile:1
#
# CI-only Dockerfile: copies a pre-built MUSL binary into a distroless image.
#
# The `build` CI job produces MUSL binaries for both amd64 and arm64.  This
# Dockerfile is used by the `docker` CI job to assemble a multi-arch image
# from those artifacts, avoiding a second compilation step.
#
# The caller stages binaries at:
#   dist/linux/amd64/snakeway
#   dist/linux/arm64/snakeway
#
# and invokes buildx with --platform linux/amd64,linux/arm64.

# ---------------------------------------------------------------------------
# Stage 1: select the correct binary for the target platform
# ---------------------------------------------------------------------------
FROM busybox:1.36 AS prepare

ARG TARGETARCH
COPY dist/linux/ /binaries/
RUN cp /binaries/${TARGETARCH}/snakeway /snakeway && chmod 0755 /snakeway

# ---------------------------------------------------------------------------
# Stage 2: runtime (distroless)
# ---------------------------------------------------------------------------
FROM gcr.io/distroless/static-debian12:nonroot

COPY --from=prepare /snakeway /usr/local/bin/snakeway
COPY packaging/config/ /etc/snakeway/

EXPOSE 80 443

ENTRYPOINT ["/usr/local/bin/snakeway"]
CMD ["run", "--config", "/etc/snakeway"]
