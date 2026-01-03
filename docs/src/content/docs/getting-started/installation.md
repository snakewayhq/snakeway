---
title: Installation
---


Snakeway is a modern, high-performance edge proxy built on Rust and Pingora. Getting it running on your local machine is
straightforward, whether you're building from source or using a pre-compiled binary.

### Building From Source

To build Snakeway from source, you will need the Rust toolchain installed. If you don't have it yet, you can install it
via [rustup](https://rustup.rs/).

Once Rust is ready, clone the Snakeway repository and build the project using Cargo:

```bash
git clone https://github.com/snakewayhq/snakeway.git
cd snakeway
cargo build --release
```

After the build completes, you'll find the `snakeway` binary in the `target/release` directory.

### The Snakeway Binary

The `snakeway` binary is a self-contained executable. However, it expects a configuration directory to be present to
define its behavior. By default, it looks for a directory named `config` in the current working directory.

A typical Snakeway deployment consists of:

1. The `snakeway` binary.
2. A `config` directory containing `snakeway.toml` and various `.d` directories for modular configuration.

### Running Snakeway

To start the proxy, simply run the binary and point it to your configuration directory:

```bash
./target/release/snakeway run --config ./config
```

If your configuration is valid, Snakeway will start and begin listening for incoming traffic based on your defined
listeners.

### Using the Justfile

The Snakeway repository includes a `Justfile` (using the [just](https://github.com/casey/just) command runner) to
simplify common development and build tasks.

If you have `just` installed, you can use these convenient commands:

- `just build`: Build the binary in debug mode.
- `just release`: Build an optimized release binary.
- `just run`: Run Snakeway with the default configuration.
- `just test`: Run the core test suite.

For a full list of available commands, you can run `just --list`.
