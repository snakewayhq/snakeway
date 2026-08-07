---
title: Overview
---

This section describes how to contribute to Snakeway itself.
It covers the repository layout, the development workflow, and the conventions that keep the codebase consistent.

Read this page first, then consult the page that matches the kind of change you are making.

## Repository Layout

```shell
.
├── crates
│   ├── snakeway/                     # Application crate (lib + bin): CLI, control plane, server
│   ├── snakeway-acme/                # ACME certificate management
│   ├── snakeway-conf/                # Configuration library crate (parsing and lowering)
│   ├── snakeway-engine/              # Request execution engine and runtime state
│   ├── snakeway-jwt-auth-device/     # Example WASM device library crate
│   ├── snakeway-net/                 # Connection-level filtering
│   ├── snakeway-observability/       # Logging, metrics, and tracing
│   ├── snakeway-tests/               # Integration test crate (separate binary, uses nextest)
│   ├── snakeway-wasm-test-device/    # WASM integration test fixture device
│   └── snakeway-wit/                 # WIT bindings for the WASM device interface
├── dev/                              # Dev-related (just recipes, k6 load testing scripts, etc.)
├── docs/                             # Docusaurus website.
├── packaging/                        # Contains all the packaging config and scripts for Docker, systemd, etc.
```

## Prerequisites

- A recent stable Rust toolchain with `rustfmt` and `clippy` components.
- [just](https://github.com/casey/just) for running project recipes (`brew install just` on macOS).
- [cargo-nextest](https://nexte.st/) for running the test suites.
- Docker, required only for the ACME integration tests (Pebble CA).
- The Protocol Buffers compiler (`protoc`), required to build the gRPC stubs in `crates/snakeway-tests` (`brew install protobuf` on macOS).
- [Zig](https://ziglang.org/) and [cargo-zigbuild](https://github.com/rust-cross/cargo-zigbuild), required only for static Linux cross-builds (`brew install zig` and `cargo install cargo-zigbuild`).
- Node.js and npm, required only for working on the documentation site.

Run `just -l` to see every available recipe.
The recipes themselves are good context for what the project can do.

## Key Commands

| Task                           | Command                   |
|--------------------------------|---------------------------|
| Build                          | `cargo build`             |
| Type-check                     | `just check`              |
| Run unit tests                 | `just test`               |
| Unit tests with coverage       | `just test-with-coverage` |
| Run integration tests          | `just integration-test`   |
| Run all tests, lint, and bench | `just test-everything`    |
| Format code                    | `just fmt`                |
| Lint (clippy)                  | `just clippy`             |
| Format and lint                | `just lint`               |
| Run locally                    | `cargo run -p snakeway`   |
| Preview the docs               | `just docs`               |
| Run microbenchmarks            | `just bench`              |

For a release build, add `-r`:

```shell
cargo build -r -p snakeway
```

## Cross-Compilation

Snakeway targets Linux environments and supports fully static musl builds for both x86_64 and aarch64.
These builds work on macOS ARM, macOS Intel, and Linux hosts.

Cross-compilation uses:

1. `cargo-zigbuild` as the build runner.
2. Zig as the C and C++ compiler and linker.
3. A `.cargo/config.toml` that disables vendored C libraries in dependent crates, including zlib-ng-sys.

This approach removes the need for external cross toolchains, wrapper binaries, or Docker images.
Zig handles all linking and provides compatible libc implementations for musl targets.

The repository's `.cargo/config.toml`:

1. Selects Zig as the C compiler, C++ compiler, and linker.
2. Disables all vendored zlib-ng builds in crates that would otherwise invoke CMake.
3. Configures the proper linker for the aarch64 and x86_64 musl targets.

:::caution
Do not modify `.cargo/config.toml` unless you are adding a new cross target.
:::

### Building static Linux binaries

```shell
# ARM64 Linux (aarch64)
just musl-aarch64

# x86_64 Linux
just musl-x86_64

# Both architectures
just musl-all
```

The static binaries land under:

```
target/<triple>/release/snakeway
```

These binaries are suitable for distribution and container deployment.

## Contribution Workflow

1. Make your change in the appropriate crate.
2. Add or update tests.
   See [Unit Tests](unit-tests.md), [Integration Tests](integration-tests.md), and [HTTP Replay Tests](http-replay-tests.md).
3. Run `just lint` and fix every diagnostic.
   See [Code Style](code-style.md).
4. If the change touches configuration, follow the full recipe in [Adding a Config Setting](adding-config-settings.md).
5. If the change adds or alters user-facing behavior, update the documentation site.
   See [Writing Documentation](writing-documentation.md).
6. Run `just test-everything` before opening a pull request.

:::tip
Scope your checks while iterating.
`cargo check -p snakeway -p snakeway` is much faster than a full workspace build and covers most changes.
:::

A pull request for a feature or bug fix typically includes:

1. New unit tests.
2. New integration tests.
3. Updated docs.
4. The code change itself.

Unsafe Rust requires clear justification and should be avoided unless necessary.

## Submitting Changes

1. Fork the repository.
2. Create a branch with a descriptive name.
3. Ensure all linting, formatting, tests, and builds succeed, including the musl cross builds.
4. Submit a pull request with a clear description of the change.

## Reporting Issues

Please include:

1. The operating system and architecture.
2. The Snakeway version or commit hash.
3. Steps to reproduce the issue.
4. Logs where relevant.

## What Each Page Covers

- **[Code Style](code-style.md)**: formatting and lint requirements that match CI.
- **[Unit Tests](unit-tests.md)**: conventions for inline unit tests in the library crates.
- **[Integration Tests](integration-tests.md)**: spinning up a real server and exercising it over the network.
- **[HTTP Replay Tests](http-replay-tests.md)**: byte-level protocol tests driven by raw fixture files.
- **[Adding a Config Setting](adding-config-settings.md)**: the end-to-end recipe for the configuration subsystem.
- **[Benchmarks](benchmarks.md)**: writing Criterion microbenchmarks for hot paths.
- **[Writing Documentation](writing-documentation.md)**: structure and style rules for this site.
- **[Mermaid Diagrams](mermaid-diagrams.md)**: authoring theme-correct diagrams for the docs.
