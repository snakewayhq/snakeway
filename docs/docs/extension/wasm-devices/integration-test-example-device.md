---
title: Integration Test Example Device
---

The **integration test example device** (`crates/snakeway-wasm-test-device`) is a small WASM device that exercises every
device capability. Snakeway's integration tests load it as a fixture, and it doubles as a working reference for
authoring your own device.

:::note
This device exists for testing and learning.
Its behavior is driven by the request path and a `mode` config value so that tests can trigger each capability on
demand.
It is not meant for production traffic.
:::

## Building

```shell
cargo component build --release --target wasm32-unknown-unknown -p snakeway-wasm-test-device
```

## Configuration Example

```hcl
wasm_devices = [
  {
    name        = "test-device"
    enable      = true
    path        = "/path/to/snakeway_wasm_test_device.wasm"
    fail_policy = "open"

    config = {
      mode       = "inject"
      echo_value = "hello-from-config"
    }
  }
]
```

## Path-Driven Behavior

The `on_request` hook branches on the request path, so a test can trigger a specific capability by choosing the path:

| Path             | Behavior                                                                 |
|------------------|--------------------------------------------------------------------------|
| `/block`         | Returns `403 Forbidden` (an `Action::Block`).                            |
| `/synthetic`     | Returns a synthetic `299` response with body `synthetic-ok`.             |
| `/rewrite`       | Rewrites the route path to `/api`.                                       |
| `/set-header`    | Sets the request header `x-wasm-test: injected`.                         |
| `/append-header` | Appends `x-multi: a` and `x-multi: b`.                                   |
| `/remove-header` | Removes the request header `x-to-remove`.                                |
| `/config-echo`   | Returns `200` with the value of the `echo_value` config key as the body. |
| any other path   | Continues unmodified.                                                    |

## Mode-Driven Behavior

The remaining hooks branch on the `mode` config value:

| Hook                             | `mode`                  | Behavior                                                                                  |
|----------------------------------|-------------------------|-------------------------------------------------------------------------------------------|
| `on_stream_request_body`         | `replace`               | Replaces each request body chunk with `replaced`.                                         |
| `on_stream_request_body`         | `drop`                  | Drops each request body chunk.                                                            |
| `before_proxy`                   | `inject`                | Sets the request header `x-before-proxy: injected`.                                       |
| `after_proxy`                    | `set-status`            | Overrides the response status to `299`.                                                   |
| `after_proxy` then `on_response` | `strip-response-header` | Sets `x-remove-me` on the response, then removes it, to exercise response-header removal. |
| `on_response`                    | `tag-response`          | Sets the response header `x-wasm-response: tagged`.                                       |

Independent of `mode`, `on_stream_request_body` blocks any request whose body contains the bytes `BLOCK_BODY`.

## Config Keys

| Key          | Description                                                                                |
|--------------|--------------------------------------------------------------------------------------------|
| `mode`       | Selects the behavior for the body, `before_proxy`, `after_proxy`, and `on_response` hooks. |
| `echo_value` | Value returned in the body by the `/config-echo` path.                                     |

## See Also

- [Authoring WASM Devices](../authoring-wasm-devices.md) for the full authoring guide.
- [JWT Auth WASM Device](./jwt-auth-device.md) for a security-focused example.
