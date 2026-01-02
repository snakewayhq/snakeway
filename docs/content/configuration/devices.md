# Configuring Devices

Devices are the primary way to extend Snakeway's functionality. They are small, composable units of logic that execute
at specific points in the request/response lifecycle. In this guide, we'll explore how to enable and configure both
built-in and WebAssembly-based devices.

### Enabling Devices

Devices are defined in your configuration using the `[[wasm]]`, `[[identity]]`, or `[[structured_logging]]` arrays.
While you can define these directly in `snakeway.toml`, they are typically placed in the `devices.d/` directory for
better organization.

### Identity Device

The `Identity` device is responsible for identifying the client making the request. It can extract client IP addresses (
respecting trusted proxies), parse User-Agent strings, and perform GeoIP lookups.

```toml
[[identity]]
enable = true
trusted_proxies = ["10.0.0.0/8", "192.168.0.0/16"]
enable_user_agent = true
ua_engine = "woothee"
enable_geoip = true
geoip_db = "/etc/snakeway/geoip.mmdb"
```

### Structured Logging Device

The `StructuredLogging` device provides detailed, JSON-formatted logs for every request. It allows you to control
exactly what information is logged, including specific headers and identity information.

```toml
[[structured_logging]]
enable = true
level = "info"
include_headers = true
allowed_headers = ["User-Agent", "Accept", "X-Request-Id"]
redacted_headers = ["Authorization", "Cookie"]
include_identity = true
identity_fields = ["ip", "country", "browser"]
```

### WASM Devices

Snakeway can also load custom logic compiled to WebAssembly. This allows you to write your own traffic handlers in
languages like Rust or Go and run them with near-native performance.

To load a WASM device, you specify the path to the `.wasm` file and an optional configuration blob that will be passed
to the device at runtime.

```toml
[[wasm]]
enable = true
path = "/path/to/my_plugin.wasm"

[wasm.config]
api_key = "secret-key"
environment = "production"
```

For more information on building your own WASM devices, check out the [WASM Device guide](/devices/wasm).

### Global Scope

In the current version of Snakeway, devices are global. Once enabled, they are active for every request processed by the
server. They are executed in a deterministic order based on their type and appearance in the configuration:

1. **Identity**: Runs first to establish client context.
2. **WASM & Built-in**: Executed in the order they are defined.
3. **Structured Logging**: Typically runs last to capture the final state of the request and response.
