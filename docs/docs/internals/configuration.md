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
      |
      |  hcl::from_str()
      v
  Spec types              (the operator's intent, as parsed)
      |
      |  validate_spec()
      v
  ValidationReport        (semantic errors and warnings, aggregated)
      |
      |  lower_configs()
      v
  Config types            (the executable runtime representation)
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
pub fn load_config(root: &Path) -> Result<RuntimeConfig, ConfigError>
```

## Spec types vs Config types

Every setting exists in two parallel structs:

| Layer      | Location                                        | Derives       | Purpose                                                |
|------------|-------------------------------------------------|---------------|--------------------------------------------------------|
| **Spec**   | `crates/snakeway-conf/src/types/specification/` | `Deserialize` | Populated directly from HCL; uses user-friendly types  |
| **Config** | `crates/snakeway-conf/src/types/runtime/`       | `Serialize`   | Resolved, executable form used by the proxy at runtime |

The conversion between them is always a `From<FooSpec> for FooConfig` or
`TryFrom<FooSpec> for FooConfig` impl that lives **in the runtime module** alongside
`FooConfig`. This keeps the lowering logic close to the type that benefits from it.

### Type selection principle

Spec and Config types serve different stages of the pipeline and follow different rules for
which Rust types to use:

**Spec types use the rawest type that deserializes infallibly from user input.** Strings,
numbers, bools, paths. These are types where serde will never reject the HCL value. This keeps the
parsing stage permissive and lets the validation stage do all semantic checking with proper
error collection. If a spec field used a parsed type like `IpNet` or `Method`, a malformed
value would cause a hard serde deserialization error (single error, stops processing) instead
of being collected into the `ValidationReport` alongside all other problems.

All integer fields in spec types use the `HclInt` alias (`i64`).
HCL's native integer type is a signed 64-bit integer, so `HclInt` accepts any value the parser can produce, including
negatives and values outside the target range.
The validation layer checks the range with full origin context, and the lowering layer narrows to the correct unsigned
type via `as` casts.

**Config types use the fully parsed, typed form.** `IpNet`, `Method`, `HeaderName`,
`SocketAddr`. This is the executable representation. Downstream code should never need to
re-parse a string it received from config.

**Lowering is the natural parsing boundary.** The `TryFrom` conversion transforms
raw-but-validated spec values into typed config values. The validation stage and the lowering
stage both parse the same strings, but for different purposes: validation parses to *check*
(read-only, collecting errors into `ValidationReport`), while lowering parses to *store*
(producing typed values for the runtime). This intentional duplication is a safety net --
if validation passes, lowering should never fail. If it does, the `TryFrom` error catches
the bug gracefully rather than panicking during a live config reload.

A few concrete differences between the layers:

| Setting      | Spec type                                                               | Config type                                               |
|--------------|-------------------------------------------------------------------------|-----------------------------------------------------------|
| Bind address | `BindInterfaceInput` (symbolic: `"loopback"`, `"all"`, or an IP string) | Resolved `SocketAddr` string                              |
| TLS          | `TlsTerminationSpec` (enum: `Manual` or `Acme`)                         | `TlsTerminationConfig` with resolved cert paths           |
| Upstreams    | Mixed `UpstreamSpec` (either `sock` or `endpoint`)                      | Separated into `UpstreamUnixConfig` / `UpstreamTcpConfig` |
| Services     | Array inside `IngressSpec`                                              | Flattened into `HashMap<String, ServiceConfig>`           |
| CIDR lists   | `Vec<String>` (raw CIDR notation)                                       | `Vec<IpNet>` (parsed network addresses)                   |
| HTTP methods | `Vec<String>` (raw method names)                                        | `Vec<Method>` (parsed HTTP methods)                       |

## Origin tracking

Every Spec struct carries an `origin: HclOrigin` field tagged `#[serde(skip)]`. `HclOrigin`
implements the `confval::Origin` trait and records the source file, section name, and
optional array index:

```rust
pub struct HclOrigin {
    pub file: PathBuf,       // e.g. "ingress.d/api.hcl"
    pub section: String,     // e.g. "service"
    pub index: Option<usize>,   // e.g. Some(0) for the first service block
}
```

The loader injects `HclOrigin` values immediately after deserialisation: before any
validation runs. Every `ValidationIssue` carries the `HclOrigin` of the spec that triggered
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

## Validation

The validation infrastructure is split across two crates:

- **`confval`** provides the generic, reusable primitives: `ValidationReport<O>`,
  `ValidationIssue<O>`, `ValidateSpec<O>`, `RangeConstraint<T>`, and rendering. See
  [confval](/internals/confval) for details.
- **`snakeway-conf`** provides domain-specific validation: the `HclOrigin` type, issue
  factory functions, and all the concrete `ValidateSpec<HclOrigin>` implementations.

### ValidateSpec trait

Spec types implement the `ValidateSpec<HclOrigin>` trait (from confval) to validate their
own field-local invariants:

```rust
pub trait ValidateSpec<O: Origin> {
    fn validate(&self, origin: &O, report: &mut ValidationReport<O>);
}
```

**Field-local** means single-field checks: range validation, format checks, path existence.
These implementations live alongside the spec types in `*_validation.rs` files (e.g.
`crates/snakeway-conf/src/types/specification/server/server_validation.rs`).

**Cross-field** checks (e.g. "HTTP/2 requires TLS") and **cross-file** checks (e.g.
"duplicate bind addresses across ingresses") remain in the centralized validators under
`validation/single_file/` and `validation/multi_file/`. The centralized validators call
`spec.validate(origin, report)` first to run field-local checks, then perform their own
relational checks.

### Orchestration

`validate_spec` in `crates/snakeway-conf/src/validation/validate.rs` is the single
orchestration point:

```rust
pub fn validate_spec(
    server: &ServerSpec,
    ingresses: &[IngressSpec],
    devices: &[DeviceSpec],
) -> ValidationReport<HclOrigin>
```

Internally it runs two categories of checks:

**Single-file validation.** Each spec is validated in isolation against its own fields.
Validators live in `crates/snakeway-conf/src/validation/single_file/`:

- `validate_ingresses`: bind address validity and uniqueness, TLS cert/key file existence,
  HTTP/2 and TLS dependency, redirect and TLS dependency, redirect status codes,
  connection filter CIDR syntax, upstream weights and addresses, route host lists, WebSocket
  and HTTP/2 conflict.
- `validate_devices`: WASM file path existence, GeoIP database paths, trusted proxy CIDR
  syntax, network policy allow list, HTTP method and header name syntax.

**Multi-file validation.** Some invariants span multiple files and can only be checked once
the full set is known. These live in `crates/snakeway-conf/src/validation/multi_file/`:

- `validate_tls`: if any ingress uses ACME TLS, `server.tls_automation` must be
  configured. If `server.tls_automation` is configured but no ingress has a TLS listener,
  a warning is emitted.

### Issue factory functions

Each validation domain has a sibling `*_issues.rs` file that contains named factory
functions returning `ValidationIssue<HclOrigin>`:

```rust
// in server_issues.rs
pub(crate) fn invalid_config_version(version: &i64, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid config version: {}", version), origin.clone())
}
```

Validators call these via `report.push(server_issues::invalid_config_version(...))`.
This keeps error messages named, greppable, and co-located with the validation logic
that produces them.

Issue files are organized by domain:

| Issues file                                   | Domain                                   |
|-----------------------------------------------|------------------------------------------|
| `server/server_issues.rs`                     | Server, TLS automation, observability    |
| `ingress/ingress_issues.rs`                   | Ingress-level checks                     |
| `ingress/bind/bind_issues.rs`                 | Bind, TLS termination, connection filter |
| `ingress/bind_admin/admin_auth_issues.rs`     | Admin authentication                     |
| `ingress/service/service_issues.rs`           | Services, upstreams, routes              |
| `ingress/static_files/static_route_issues.rs` | Static file serving                      |
| `device/device_issues.rs`                     | All device types                         |

### Rendering

When validation fails, the `ValidationReport` is rendered for the operator. Three
output formats are supported, all provided by the confval crate:

- **`render_pretty`**: colorized, grouped by source file (default for `snakeway config check`)
- **`render_plain`**: one line per issue, for CI/scripts
- **`render_json`**: structured JSON for tooling

## ConfigError vs ValidationReport

`ConfigError` and `ValidationReport` serve different roles:

| Type               | When used                                      | What triggers it                                                           |
|--------------------|------------------------------------------------|----------------------------------------------------------------------------|
| `ConfigError`      | Hard failure; `load_config` returns `Err(...)` | File I/O failure, HCL syntax error, unresolvable address during lowering   |
| `ValidationReport` | Soft accumulation inside `ConfigError`         | Semantic violations (wrong value, missing field, cross-file inconsistency) |

A `ConfigError::SemanticValidationFailed` wraps the `ValidationReport` when any errors
are found. Warnings alone do not block startup.

## Key files at a glance

| File                           | Responsibility                                                          |
|--------------------------------|-------------------------------------------------------------------------|
| `confval/src/`                 | Generic validation primitives (`ValidationReport`, `ValidateSpec`, etc) |
| `conf/loader.rs`               | Entry point: `load_config`, `load_spec_files`, `load_config_from_specs` |
| `conf/discover.rs`             | Glob-based file discovery                                               |
| `conf/parse.rs`                | `parse_devices`, `parse_ingress`: HCL to Spec                           |
| `conf/lower.rs`                | `lower_configs`: Spec to Config                                         |
| `conf/types/specification/`    | All `*Spec` structs, `*_validation.rs`, and `*_issues.rs` files         |
| `conf/types/runtime/`          | All `*Config` structs and their `From`/`TryFrom` impls                  |
| `conf/validation/validate.rs`  | `validate_spec` orchestrator                                            |
| `conf/validation/single_file/` | Per-file validators                                                     |
| `conf/validation/multi_file/`  | Cross-file validators                                                   |
| `conf/validation/error.rs`     | `ConfigError` enum                                                      |
