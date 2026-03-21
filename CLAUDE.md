# Snakeway — Claude Code Guide

## Project Overview

Snakeway is a Rust-based reverse proxy built on top of [Pingora](https://github.com/cloudflare/pingora).
It supports HTTP/HTTPS proxying, WebSocket proxying, gRPC proxying, static file serving, ACME/TLS automation,
and a composable "device" middleware pipeline.

## Repository Layout

```
crates/snakeway/           # Main binary crate (entrypoint, CLI wiring)
crates/snakeway-conf/      # Configuration library crate — handles configuration file parsing and lowering to runtime representation.
crates/snakeway-core/      # Core library crate — all business logic lives here
crates/snakeway-wit/       # WIT bindings for WASM device interface
tests/integration/         # Integration test crate (separate binary, uses nextest)
docs/                      # Docusaurus documentation site
k6/                        # k6 load test scripts
```

## Key Commands

| Task                  | Command                      |
|-----------------------|------------------------------|
| Build                 | `just build` / `cargo build` |
| Run unit tests        | `just test`                  |
| Run integration tests | `just integration-test`      |
| Run all tests + lint  | `just test-everything`       |
| Format code           | `just fmt` / `cargo fmt`     |
| Lint (clippy)         | `just clippy`                |
| Format + lint         | `just lint`                  |
| Run locally           | `just run`                   |

## Code Style

- **Always run `cargo fmt` and `cargo clippy --all-targets --all-features -- -D warnings`** after making changes.
  Clippy warnings are treated as errors in CI. See the `lint` skill for the full workflow.
- Use `just lint` as a convenient shorthand for both steps.

## Unit Testing (`snakeway-core`)

Unit tests live inside `snakeway-core` in `#[cfg(test)] mod tests;` submodules.
All tests follow the **Arrange / Act / Assert (AAA)** pattern with explicit `// Arrange`, `// Act`, `// Assert`
comments.
See the `unit-test` skill for full conventions.

## Integration Testing (`tests/integration`)

Integration tests live in `tests/integration` organised by feature area.
They spin up a real Snakeway server with a real upstream and make live HTTP/WS/gRPC requests.

Two sub-types:

- **Standard integration tests** — use `TestServer` + `reqwest::blocking::Client`. See the `integration-test` skill.
- **HTTP replay tests** — replay raw `.http` fixture files over a TCP socket. See the `http-replay-test` skill.

## Configuration

Snakeway config is loaded from HCL files. The integration-test crate also provides a
`ConfigBuilder` for constructing `RuntimeConfig` values entirely in Rust without touching the filesystem.
See the `conf-settings` skill for the end-to-end recipe when adding a new setting.

## Benchmarking

Criterion microbenchmarks live in `crates/snakeway-core/benches/`. Run with `just bench`.
See the `benchmarking` skill for conventions and templates.

## Documentation

The documentation site lives in `docs/` and uses Docusaurus. Preview with `just docs`.
When adding or changing config fields, update the corresponding docs page.
See the `docs` skill for the spec-to-docs mapping, writing conventions, and recipes.

## Environment constraints

### `protoc` is not installed

The `tests/integration` crate requires the Protocol Buffers compiler (`protoc`) to
compile its gRPC stubs. **`protoc` is not available in this environment and cannot
be installed.** Do not attempt to install it.

Consequence: `cargo check --workspace` and `cargo test --workspace` will always fail
for the `tests/integration` crate with:

```
thread 'main' panicked at tests/integration/build.rs:...:
failed to compile protos: Custom { kind: NotFound, error: "Could not find `protoc`" }
```

**This is a pre-existing environment issue, not caused by code changes.**

To verify that your own changes compile correctly, scope the check to the relevant
packages instead:

```sh
cargo check -p snakeway-core -p snakeway
cargo test  -p snakeway-core -p snakeway
```
