# Contributing to Snakeway

This document outlines the development workflow, code standards, and the cross-compilation setup required for building Snakeway as a static Linux binary.

## Prerequisites

The project uses the following tools:

1. Rust toolchain (rustup)
2. Zig 0.12 or later (Install using Homebrew on macOS: `brew install zig`)
3. cargo-zigbuild (Install with `cargo install cargo-zigbuild`)
4. Just command runner (Install using Homebrew on macOS `brew install just`)

Zig is used for cross-compilation to avoid external C toolchains and to provide consistent builds across environments.

## Project Layout

The repository uses a standard Rust workspace layout. The Snakeway binary source is located in src, and configuration examples are in the config directory.

## Building Locally

To build a non-cross-compiled binary:

```shell
cargo build
```

Release mode:

```shell
cargo build --release
```

To run Snakeway locally with the default configuration:

```shell
just run
```

## Cross Compilation Overview

Snakeway targets Linux environments and supports fully static musl builds for both x86_64 and aarch64 architectures. 

These builds work on macOS ARM, macOS Intel, and Linux hosts.

Cross compilation uses:

1. `cargo-zigbuild` as the build runner
2. Zig as the C and C++ compiler
3. A `.cargo/config.toml` configuration that disables vendored C libraries in dependent crates, including zlib-ng-sys

This approach removes the need for external cross toolchains, wrapper binaries, or Docker images. 
Zig handles all linking and provides compatible libc implementations for musl targets.

`.cargo/config.toml`

The repository contains a `.cargo/config.toml` file that:

1. Selects Zig as the C compiler, C++ compiler, and linker
2. Disables all vendored zlib-ng builds in crates that would otherwise invoke CMake
3. Configures the proper linker for aarch64 and x86_64 musl targets

Developers should not modify this file unless adding new cross targets.

### Building Static Linux Binaries

Use the provided Justfile to produce musl binaries:

ARM64 Linux (aarch64):

```shell
just musl-aarch64
```

x86_64 Linux:

```shell
just musl-x86_64
```

Both architectures:

```shell
just musl-all
```

This produces static binaries under:

```shell
target/<triple>/release/snakeway
```

These binaries are suitable for distribution and container deployment.

## Code Style and Quality

1. Format all code before submitting a pull request.
2. Clippy must succeed with the default lint settings.
3. Unsafe Rust requires clear justification and should be avoided unless necessary.

Run formatting and linting:

```shell
just lint
```

## Submitting Changes

1. Fork the repository.
2. Create a branch with a descriptive name.
3. Ensure all linting, formatting, and builds succeed, including musl cross builds.
4. Submit a pull request with a clear description of the change.

## Reporting Issues

Please include:

1. The operating system and architecture
2. The Snakeway version or commit hash
3. Steps to reproduce the issue
4. Logs where relevant