---
title: Overview
---

This section describes how to contribute to Snakeway itself.
It covers the repository layout, the development workflow, and the conventions that keep the codebase consistent.

Read this page first, then consult the page that matches the kind of change you are making.

## Repository Layout

```
crates/snakeway/           # Main binary crate (entrypoint, CLI wiring)
crates/snakeway-conf/      # Configuration library crate (parsing and lowering)
crates/snakeway-core/      # Core library crate (all business logic)
crates/snakeway-wit/       # WIT bindings for the WASM device interface
crates/snakeway-tests/     # Integration test crate (separate binary, uses nextest)
docs/                      # This documentation site (Docusaurus)
k6/                        # k6 load test scripts
```

## Prerequisites

- A recent stable Rust toolchain with `rustfmt` and `clippy` components.
- [just](https://github.com/casey/just) for running project recipes.
- [cargo-nextest](https://nexte.st/) for running the test suites.
- Docker, required only for the ACME integration tests (Pebble CA).
- The Protocol Buffers compiler (`protoc`), required to build the gRPC stubs in `crates/snakeway-tests`.
- Node.js and npm, required only for working on the documentation site.

Run `just -l` to see every available recipe.
The recipes themselves are good context for what the project can do.

## Key Commands

| Task                  | Command                      |
|-----------------------|------------------------------|
| Build                 | `just build` or `cargo build`|
| Run unit tests        | `just test`                  |
| Run integration tests | `just integration-test`      |
| Run all tests and lint| `just test-everything`       |
| Format code           | `just fmt`                   |
| Lint (clippy)         | `just clippy`                |
| Format and lint       | `just lint`                  |
| Run locally           | `just run`                   |
| Preview the docs      | `just docs`                  |
| Run microbenchmarks   | `just bench`                 |

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
`cargo check -p snakeway-core -p snakeway` is much faster than a full workspace build and covers most changes.
:::

## What Each Page Covers

- **[Code Style](code-style.md)**: formatting and lint requirements that match CI.
- **[Unit Tests](unit-tests.md)**: conventions for inline unit tests in the library crates.
- **[Integration Tests](integration-tests.md)**: spinning up a real server and exercising it over the network.
- **[HTTP Replay Tests](http-replay-tests.md)**: byte-level protocol tests driven by raw fixture files.
- **[Adding a Config Setting](adding-config-settings.md)**: the end-to-end recipe for the configuration subsystem.
- **[Benchmarks](benchmarks.md)**: writing Criterion microbenchmarks for hot paths.
- **[Writing Documentation](writing-documentation.md)**: structure and style rules for this site.
- **[Mermaid Diagrams](mermaid-diagrams.md)**: authoring theme-correct diagrams for the docs.
