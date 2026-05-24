---
title: confval
---

**confval** is a standalone Rust crate for structured configuration validation.
It lives in `crates/confval/` and provides the generic primitives that `snakeway-conf` builds on.

## Design goals

- **Framework-agnostic.** Works with any config format (HCL, TOML, YAML, JSON, env vars).
- **Zero opinion on parsing.** You parse however you want; confval validates the result.
- **Minimal dependencies.** Core functionality has zero dependencies. `serde` and `owo-colors` are optional, behind
  feature flags.
- **Zero-cost generics.** Generic over `Origin` so each project defines its own source-location type.

## Core types

### Origin

The trait that source-location types implement.
confval never dictates what a "location" looks like.
It only requires that the type can be displayed and grouped.

```rust
pub trait Origin: Display + Debug + Clone {
    fn source(&self) -> &str;
}
```

Snakeway implements this with `HclOrigin`, which carries the file path, section name, and optional block index.
confval also provides `SimpleOrigin` for tests and examples.

### ValidationIssue

A single validation finding.
Constructed by validators, collected by the report.

```rust
pub struct ValidationIssue<O: Origin> {
    pub severity: Severity,
    pub message: String,
    pub origin: O,
    pub help: Option<String>,
}
```

Four constructors cover the common patterns:

```rust
ValidationIssue::error("message", origin)
ValidationIssue::error_with_help("message", origin, "help text")
ValidationIssue::warning("message", origin)
ValidationIssue::warning_with_help("message", origin, "help text")
```

### ValidationReport

A collector with no domain-specific methods.

```rust
pub struct ValidationReport<O: Origin> {
    /* errors, warnings */
}
```

Key methods:

- `error(issue)`, `warning(issue)`, `push(issue)`: collect issues (`push` routes by severity)
- `has_errors()`, `has_warnings()`, `has_issues()`: query state
- `errors()`, `warnings()`, `iter()`: access issues
- `merge(other)`: combine two reports

### ValidateSpec

A convention trait for structured validation.
Spec types implement this to validate their own field-local invariants.

```rust
pub trait ValidateSpec<O: Origin> {
    fn validate(&self, origin: &O, report: &mut ValidationReport<O>);
}
```

### RangeConstraint

A reusable numeric bounds check.
Defined declaratively with a macro, validated at call sites.

```rust
range_constraint!(PORT, u16, min: 1, max: 65535);
range_constraint!(INTERVAL, u64, min: 1, max: 3600, units: "s");
range_constraint!(WORKERS, usize, min: 1, max: 128, help: "Match this to your CPU core count.");
```

The `validate_range_field!` macro checks a value and pushes an issue to the report:

```rust
validate_range_field!(PORT, self.port, report, origin);
```

When **help** is provided, it overrides the auto-generated help text for both below-min and above-max violations.
When omitted, confval generates a default message like "Set port to at least 1".

## Rendering

Renderers write to a `Write` sink rather than printing to stdout directly.
Three formats are available:

| Method          | Feature gate     | Format                             |
|-----------------|------------------|------------------------------------|
| `render_plain`  | always available | One line per issue, for CI/scripts |
| `render_pretty` | `color`          | Colorized, grouped by source       |
| `render_json`   | `serde`          | Structured JSON                    |

```rust
let mut out = String::new();
report.render_plain( & mut out).unwrap();
eprint!("{out}");
```

## Feature flags

| Flag    | Default | Brings in    | Enables                            |
|---------|---------|--------------|------------------------------------|
| `serde` | off     | `serde`      | `Serialize` derives, `render_json` |
| `color` | off     | `owo-colors` | `render_pretty` with ANSI color    |

Snakeway enables both features in its workspace dependency.

## How Snakeway uses confval

Snakeway extends confval with two domain-specific pieces:

1. **`HclOrigin`** implements `Origin` with HCL file path and section tracking.
2. **Issue factory functions** are named functions in `*_issues.rs` files that return `ValidationIssue<HclOrigin>` with
   pre-formatted messages.
   These keep error messages named and greppable while using confval's generic report.

```rust
// in server_issues.rs
pub(crate) fn invalid_config_version(version: &u32, origin: &HclOrigin) -> ValidationIssue<HclOrigin> {
    ValidationIssue::error(format!("invalid config version: {}", version), origin.clone())
}

// call site in server_validation.rs
report.push(server_issues::invalid_config_version( & self .version, origin));
```

See [Configuration Internals](/internals/configuration) for the full validation pipeline.
