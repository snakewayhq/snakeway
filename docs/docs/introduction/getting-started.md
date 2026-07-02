---
title: Getting Started
---

## What is Snakeway?

Snakeway is a **programmable traffic control engine** that sits in front of your services and executes a configurable
pipeline of logic on every request and response.

At its core, Snakeway answers a simple question:

> *"What should happen to this request before, during, and after it reaches an upstream service?"*

Snakeway lets you answer that question with **devices**, small, composable units of logic that can observe, mutate,
short-circuit, or enrich traffic as it flows through the system.

## Installation

Snakeway is a high-performance edge proxy built on Rust and Pingora. Getting it running on your local machine is
straightforward, whether you are building from source or using a pre-compiled binary.

### Installing on Ubuntu/Debian

Download the `.deb` [package](https://github.com/snakewayhq/snakeway/releases/latest) for your architecture.

For example:

```shell
sudo apt install ./snakeway_0.11.0-1_amd64.deb
```

### Building From Source

To build Snakeway from source, you will need the Rust toolchain installed. If you do not have it yet, you can install it
via [rustup](https://rustup.rs/).

Once Rust is ready, clone the Snakeway repository and build the project using Cargo:

```shell
git clone https://github.com/snakewayhq/snakeway.git
cd snakeway
cargo build --release -p snakeway
```

After the build completes, you will find the `snakeway` binary in the `target/release` directory.

## Your First Proxy

The best way to understand Snakeway is to see it in action. In this section, you will set up a minimal proxy
that forwards traffic to a public API.

### 1. Initialize Your Configuration

The `snakeway` binary is a self-contained executable. However, it expects a configuration directory to define its
behavior. By default, it looks for a directory named `config` in the current working directory. This can also be set
via the `SNAKEWAY_CONFIG` environment variable (
see [CLI reference](../administration/cli.md#snakeway_config-environment-variable)).

Generate a Snakeway configuration directory called "my-first-proxy" using the `httpbin` template:

```shell
snakeway config init ./my-first-proxy --template=httpbin
```

You should now see a directory structure that looks like this:

```shell
my-first-proxy
├── device.d
│ └── identity.hcl
├── ingress.d
│ └── httpbin.hcl
└── snakeway.hcl
```

:::note
The `httpbin` template creates a configuration that forwards `http://localhost:8080/get` requests to
`https://httpbin.org/get`.

Use `snakeway config init -h` to see other template options.
:::

### 2. Launch the Proxy

Run Snakeway, pointing it to your new configuration directory:

```shell
snakeway run --config ./my-first-proxy
```

### 3. Verify with Curl

Finally, open a new terminal and send a request to your local proxy:

```shell
curl -i http://localhost:8080/get
```

You should see a successful response from `httpbin.org`, served through your local Snakeway instance:

```http
HTTP/1.1 200 OK
Content-Type: application/json
...
{
  "args": {},
  "headers": {
    "Host": "httpbin.org",
    ...
  },
  "url": "https://httpbin.org/get"
}
```

Congratulations! You have configured and launched your first Snakeway proxy. From here, you can begin exploring more
advanced features like [Configuration](/docs/configuration/overview), [Devices](../extension/understanding-devices.md),
and [Static File Serving](../administration/static-files.md).
