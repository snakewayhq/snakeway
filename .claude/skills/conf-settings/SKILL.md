# Skill: conf-setting — Adding a New Setting to the Config Subsystem

This skill describes the `snakeway-core` configuration subsystem and provides a precise
recipe for adding a new setting. Follow every step; skipping one will produce a compile
error or a silent no-op.

---

## How the subsystem works

Config loading is a strict three-phase pipeline:

```
HCL file(s)
    │  hcl::from_str()
    ▼
Spec types          ← "what the user wrote"
    │  validate_spec()
    ▼
ValidationReport    ← semantic errors / warnings collected here
    │  lower_configs()
    ▼
Config types        ← "what the proxy actually runs"
```

The orchestrating code lives in four files:

| File | Role |
|------|------|
| `crates/snakeway-core/src/conf/loader.rs` | Entry point: reads files, calls validate, calls lower |
| `crates/snakeway-core/src/conf/validation/validate.rs` | Calls all validators, returns `ValidationReport` |
| `crates/snakeway-core/src/conf/lower.rs` | Converts every Spec type into its Config counterpart |
| `crates/snakeway-core/src/conf/validation/report.rs` | Defines `ValidationReport` and all typed error helpers |

---

## The two-type system: Spec vs Config

Every setting exists in **two parallel structs**:

- **Spec** (`crates/snakeway-core/src/conf/types/specification/`) — derives `Deserialize`; populated
  directly from HCL. Fields use user-friendly types (`String`, `PathBuf`, `Option<…>`).
  The `origin: Origin` field (skip-serialized) carries the source file location for error messages.

- **Config** (`crates/snakeway-core/src/conf/types/runtime/`) — the resolved, executable form.
  Populated by lowering. May use resolved types (e.g. `SocketAddr` instead of `String`,
  or a concrete enum instead of a raw string).

The conversion is always a `From<FooSpec> for FooConfig` or `TryFrom<FooSpec> for FooConfig`
impl that lives **in the runtime file alongside `FooConfig`**.

---

## Recipe: adding a setting to the `server` block

This is the most common case. The server block is defined in `snakeway.hcl`.

### Step 1 — Add the field to the Spec struct

**File:** `crates/snakeway-core/src/conf/types/specification/server.rs`

Add a field to `ServerSpec`. Use `Option<T>` when the setting is optional:

```rust
pub struct ServerSpec {
    // ... existing fields ...

    /// Maximum number of simultaneous downstream connections.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max_connections: Option<u32>,
}
```

If the field has a non-`Default` default, supply it with a `#[serde(default = "fn_name")]`
attribute and a free function:

```rust
#[serde(default = "default_max_connections")]
pub max_connections: u32,

fn default_max_connections() -> u32 { 1024 }
```

Update `ServerSpec::default()` at the bottom of the same file to include the new field.

### Step 2 — Add the field to the Config struct

**File:** `crates/snakeway-core/src/conf/types/runtime/server.rs`

Add a matching field to `ServerConfig`:

```rust
pub struct ServerConfig {
    // ... existing fields ...
    pub max_connections: Option<u32>,
}
```

### Step 3 — Thread the field through the `TryFrom` conversion

Still in `crates/snakeway-core/src/conf/types/runtime/server.rs`, update the `TryFrom<ServerSpec>
for ServerConfig` impl:

```rust
impl TryFrom<ServerSpec> for ServerConfig {
    type Error = String;
    fn try_from(spec: ServerSpec) -> Result<Self, Self::Error> {
        Ok(Self {
            // ... existing fields ...
            max_connections: spec.max_connections,
        })
    }
}
```

If the field requires non-trivial conversion (e.g. `PathBuf` → `String`), do it here and
return `Err(…)` if it fails. The error string is wrapped in `ConfigError::InvalidServerConfig`
by the caller in `lower.rs` — no changes to `lower.rs` are needed for server fields.

### Step 4 — Add validation (if needed)

**File:** `crates/snakeway-core/src/conf/validation/single_file/server.rs`

Add your check inside `validate_server()`:

```rust
pub fn validate_server(server_spec: &ServerSpec, report: &mut ValidationReport) {
    // ... existing checks ...

    if let Some(max) = server_spec.max_connections {
        if max == 0 {
            report.error(
                "max_connections must be greater than zero".to_string(),
                &server_spec.origin,
                Some("Set max_connections to a positive integer.".to_string()),
            );
        }
    }
}
```

For reusable range checks, see `validate_range()` in
`crates/snakeway-core/src/conf/validation/validator/mod.rs` and the `SERVER_THREADS` constant for
a pattern to follow.

**Add a typed helper to `ValidationReport`** when the error should be reusable across
validators. Add a new `impl ValidationReport` block in
`crates/snakeway-core/src/conf/validation/report.rs`:

```rust
/// Server Spec Validation
impl ValidationReport {
    // ... existing helpers ...

    pub fn invalid_max_connections(&mut self, value: u32, origin: &Origin) {
        self.error(
            format!("max_connections must be > 0, got {}", value),
            origin,
            None,
        );
    }
}
```

Then call `report.invalid_max_connections(max, &server_spec.origin)` from the validator.

### Step 5 — Verify

```sh
cargo check -p snakeway-core -p snakeway
cargo test  -p snakeway-core -p snakeway
just lint
```

---

## Variant: adding a setting to an ingress block

Ingress settings live in `ingress.d/*.hcl`. The same four steps apply but the files differ:

| Step | File |
|------|------|
| Spec field | `crates/snakeway-core/src/conf/types/specification/ingress.rs` (or `bind/`, `service/`, etc.) |
| Config field | The matching file under `crates/snakeway-core/src/conf/types/runtime/` |
| Conversion | `From`/`TryFrom` impl in the runtime file |
| Validation | `crates/snakeway-core/src/conf/validation/single_file/ingress.rs` — `validate_ingress()` or one of its sub-functions |
| Lowering | `crates/snakeway-core/src/conf/lower.rs` — update the relevant `lower_configs()` section (listeners, services, static files) to plumb the new field through |

Note that `lower.rs` **does** need updating for ingress fields because the lowering loop
there explicitly constructs `ListenerConfig`, `ServiceConfig`, etc.

---

## Variant: settings that need cross-file (multi-file) validation

Some rules span multiple HCL files — for example, "if any ingress uses ACME TLS, the
server block must configure `tls_automation`". These checks live in
`crates/snakeway-core/src/conf/validation/multi_file/`.

Add a new function there and call it from `validate_spec()` in
`crates/snakeway-core/src/conf/validation/validate.rs`:

```rust
pub fn validate_spec(server, ingresses, devices) -> ValidationReport {
    // ...
    multi_file::validate_tls(server, ingresses, &mut report);
    multi_file::validate_my_cross_file_rule(server, ingresses, &mut report);  // ← new
    // ...
}
```

---

## Where settings appear in HCL

- **Server-level** — `snakeway.hcl`, inside the `server { … }` block.
- **Ingress-level** — any file matched by `include.ingresses`, at the top level or inside
  `bind { … }`, `services = [ { … } ]`, or `static_files = [ { … } ]`.
- **Device-level** — any file matched by `include.devices`, inside the named device block
  (e.g. `identity_device { … }`). Device spec structs live in
  `crates/snakeway-core/src/conf/types/specification/device/` and are validated in
  `crates/snakeway-core/src/conf/validation/single_file/device.rs`.
