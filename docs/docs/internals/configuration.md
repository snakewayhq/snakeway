---
title: Configuration Internals
---

The configuration subsystem lives in the `snakeway-conf` crate. It is responsible for
reading HCL files from disk, checking them for semantic correctness, and producing the
runtime types that the rest of the proxy reads.

## The three-phase pipeline

Every startup (and every config-check run) passes through three ordered phases:

```
HCL files on disk
      │
      │  hcl::from_str()
      ▼
  Spec types          ← the operator's intent, as parsed
      │
      │  validate_spec()
      ▼
  ValidationReport    ← semantic errors and warnings, aggregated
      │
      │  lower_configs()
      ▼
  Config types        ← the executable runtime representation
```

**Parse** is a hard fail. A syntax error or an unrecognised field causes `load_config` to
return a `ConfigError::Parse` immediately: no validation report is generated.

**Validate** never panics and never returns early. Every validator receives a mutable
`ValidationReport` and appends issues to it. Validators keep running even after earlier
errors are found, so the operator sees all problems in one pass.

**Lower** converts Spec types to Config types. It assumes validation already passed,
so a failure here (e.g. an unresolvable socket address) is wrapped in a `ConfigError`
variant and treated as an internal error rather than user-visible feedback.

The entry point for all three phases is `load_config` in
`crates/snakeway-conf/src/loader.rs`:

```rust
pub fn load_config(root: &Path) -> Result<ValidatedConfig, ConfigError>
```

`ValidatedConfig` bundles the fully-lowered `RuntimeConfig` together with the
`ValidationReport`. The caller (the `snakeway` binary) inspects `is_valid()` and renders
the report before deciding whether to start the server.

## Spec types vs Config types

Every setting exists in two parallel structs:

| Layer      | Location                                      | Derives       | Purpose                                                |
|------------|-----------------------------------------------|---------------|--------------------------------------------------------|
| **Spec**   | `crates/snakeway-conf/src/types/specification/` | `Deserialize` | Populated directly from HCL; uses user-friendly types  |
| **Config** | `crates/snakeway-conf/src/types/runtime/`       | `Serialize`   | Resolved, executable form used by the proxy at runtime |

The conversion between them is always a `From<FooSpec> for FooConfig` or
`TryFrom<FooSpec> for FooConfig` impl that lives **in the runtime module** alongside
`FooConfig`. This keeps the lowering logic close to the type that benefits from it.

A few concrete differences between the layers:

| Setting      | Spec type                                                               | Config type                                               |
|--------------|-------------------------------------------------------------------------|-----------------------------------------------------------|
| Bind address | `BindInterfaceInput` (symbolic: `"loopback"`, `"all"`, or an IP string) | Resolved `SocketAddr` string                              |
| TLS          | `TlsTerminationSpec` (enum: `Manual` or `Acme`)                         | `TlsTerminationConfig` with resolved cert paths           |
| Upstreams    | Mixed `UpstreamSpec` (either `sock` or `endpoint`)                      | Separated into `UpstreamUnixConfig` / `UpstreamTcpConfig` |
| Services     | Array inside `IngressSpec`                                              | Flattened into `HashMap<String, ServiceConfig>`           |

## Origin tracking

Every Spec struct carries an `origin: Origin` field tagged `#[serde(skip)]`. `Origin`
records the source file, section name, and optional array index:

```rust
pub struct Origin {
    pub(crate) file: PathBuf,     // e.g. "ingress.d/api.hcl"
    pub(crate) section: String,   // e.g. "service"
    pub(crate) index: Option<usize>, // e.g. Some(0) for the first service block
}
```

The loader injects `Origin` values immediately after deserialisation: before any
validation runs. Every `ValidationIssue` carries the `Origin` of the spec that triggered
it, so error messages can point at a specific file and block rather than just a field name.

## File discovery

`snakeway.hcl` is the entrypoint. It contains an `include` block with two glob patterns:

```hcl
server {
  version = 1
}

include {
  devices   = "device.d/*.hcl"
  ingresses = "ingress.d/*.hcl"
}
```

`discover()` in `crates/snakeway-conf/src/discover.rs` resolves each pattern relative to
the config root and returns an ordered list of paths. Ordering is deterministic
(lexicographic within each directory), which matters for listener naming.

## Validation in depth

`validate_spec` in `crates/snakeway-conf/src/validation/validate.rs` is the single
orchestration point:

```rust
pub fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceSpec],
) -> ValidationReport
```

Internally it runs two categories of checks:

### Single-file validation

Each spec is validated in isolation against its own fields. Validators live in
`crates/snakeway-conf/src/validation/single_file/`:

- `validate_version`: must be `1`. If it fails, all other validation is skipped (a
  version mismatch means the entire schema could be wrong).
- `validate_server`: PID file parent directory, CA file PEM validity, thread count range,
  ACME directory URL format, contact emails, cert store, renewal window.
- `validate_ingresses`: bind address validity and uniqueness, TLS cert/key file existence,
  HTTP/2 ↔ TLS dependency, redirect ↔ TLS dependency, redirect status codes,
  connection filter CIDR syntax, upstream weights and addresses, route host lists, WebSocket
  ↔ HTTP/2 conflict.
- `validate_devices`: WASM file path existence, GeoIP database paths, trusted proxy CIDR
  syntax, network policy allow list, HTTP method and header name syntax.

### Multi-file validation

Some invariants span multiple files and can only be checked once the full set is known.
These live in `crates/snakeway-conf/src/validation/multi_file/`:

- `validate_tls`: if any ingress uses ACME TLS, `server.tls_automation` must be
  configured. If `server.tls_automation` is configured but no ingress has a TLS listener,
  a warning is emitted.

### ValidationReport and ValidationIssue

`ValidationReport` accumulates issues through two primitives:

```rust
report.error("message", & origin, Some("help text"));
report.warning("message", & origin, None);
```

For each distinct error kind there is also a typed helper method on `ValidationReport`:
for example:

```rust
report.http2_requires_tls( & addr, & bind.origin);
report.acme_tls_requires_domains( & bind.origin);
```

These helpers keep validator code readable and keep error messages consistent across the
codebase. They are defined as `impl ValidationReport` blocks grouped by the spec they
belong to, at the bottom of `crates/snakeway-conf/src/validation/report.rs`.

`ValidationReport::is_valid()` returns `true` only when `errors` is empty. Warnings alone
do not block startup.

## ConfigError vs ValidationReport

`ConfigError` and `ValidationReport` serve different roles:

| Type               | When used                                        | What triggers it                                                           |
|--------------------|--------------------------------------------------|----------------------------------------------------------------------------|
| `ConfigError`      | Hard failure; `load_config` returns `Err(…)`     | File I/O failure, HCL syntax error, unresolvable address during lowering   |
| `ValidationReport` | Soft accumulation; `load_config` returns `Ok(…)` | Semantic violations (wrong value, missing field, cross-file inconsistency) |

A `ConfigError` means the pipeline cannot proceed at all. A non-empty `ValidationReport`
means the pipeline completed but the operator should not start the server.

## Key files at a glance

| File                                  | Responsibility                                                          |
|---------------------------------------|-------------------------------------------------------------------------|
| `conf/loader.rs`                      | Entry point: `load_config`, `load_spec_files`, `load_config_from_specs` |
| `conf/discover.rs`                    | Glob-based file discovery                                               |
| `conf/parse.rs`                       | `parse_devices`, `parse_ingress`: HCL to Spec                           |
| `conf/lower.rs`                       | `lower_configs`: Spec to Config; defines `IrConfig`                     |
| `conf/types/specification/`           | All `*Spec` structs                                                     |
| `conf/types/runtime/`                 | All `*Config` structs and their `From`/`TryFrom` impls                  |
| `conf/validation/validate.rs`         | `validate_spec` orchestrator                                            |
| `conf/validation/single_file/`        | Per-file validators                                                     |
| `conf/validation/multi_file/`         | Cross-file validators                                                   |
| `conf/validation/report.rs`           | `ValidationReport`, `ValidationIssue`, typed error helpers              |
| `conf/validation/error.rs`            | `ConfigError` enum                                                      |
| `conf/validation/validated_config.rs` | `ValidatedConfig` wrapper                                               |
