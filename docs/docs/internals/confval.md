---
title: confval
---

**confval** is a standalone Rust crate for span-first configuration parsing, validation, and lowering.
It lives in `crates/confval/` and provides the generic primitives that `snakeway-conf` builds on.

## Design goals

- **Span-first.** Every parsed value carries the byte range it came from, so any later check can point
  at the exact line and column in the source file.
- **One-pass reporting.** Parsing and validation never stop at the first problem. Issues accumulate in
  a report and the operator sees everything at once.
- **Structural and semantic separation.** Parsing only checks shape (field exists, field has the right
  type). Semantic rules (ranges, closed sets, cross-field invariants) live in plain validation functions
  that run after parsing.
- **Format-agnostic core.** The provenance module has no parser dependency. The HCL adapter is an
  optional feature; other formats can be layered on the same primitives.
- **Minimal dependencies.** The core has none. `serde`, `owo-colors`, `hcl-edit`, and the derive macros
  are all behind feature flags.

## Provenance types

The `confval::provenance` module defines the core vocabulary.

### Span and SourceId

A `Span` is a byte range inside one registered source:

```rust
pub struct Span {
    pub source: SourceId,
    pub start: u32,
    pub end: u32,
}
```

`SourceId` is a lightweight handle issued by the `SourceMap`. Spans are plain data; resolving them to
line and column numbers happens only at render time.

### SourceMap

The `SourceMap` interns source text. Each file (or in-memory string) is registered once and identified
by its `SourceId`:

```rust
let mut sources = SourceMap::new();
let id = sources.add("snakeway.hcl", text);
```

Reports do not own source text. Renderers take `&SourceMap` so the text is stored exactly once no
matter how many issues reference it.

### Located

`Located<T>` pairs a value with the span it was parsed from:

```rust
pub struct Located<T> {
    pub value: T,
    pub span: Span,
}
```

Key behaviors:

- **Value-only equality.** `PartialEq`, `Eq`, and `Hash` ignore the span, so two configs with the same
  values compare equal regardless of formatting.
- **`Deref` to `T`.** Method calls pass through to the inner value.
- **`Located::detached(value)`** produces a value with a sentinel span. This is how specs are
  constructed in code (tests, builders, generated templates) without a source file.
- **`Default`** is `detached(T::default())`.
- With the `serde` feature, `Located<T>` serializes transparently as `T` and deserializes detached.

### Report and IssueBuilder

`Report` collects issues. Validators receive `&mut Report` and emit through a builder:

```rust
report
.error("port must be between 1 and 65535")
.at(spec.port.span)
.help("Choose a port in the range 1..=65535.")
.emit();
```

- `report.error(msg)` and `report.warning(msg)` return an `IssueBuilder`.
- `.at(span)` attaches the primary span. Issues without a span render without a source location.
- `.help(text)` adds a suggestion line.
- `.related(span, label)` attaches secondary spans, used for messages like "first declared here".
- `.emit()` finalizes the issue. The builder is `#[must_use]`, so forgetting `.emit()` is a
  compile-time warning.

Query methods: `has_errors()`, `has_warnings()`, `is_empty()`. Severity is the two-variant
`Severity` enum (`Error`, `Warning`).

## Rendering

Renderers write into any `fmt::Write` sink and take the `SourceMap` to resolve spans:

| Method          | Feature gate     | Format                                          |
|-----------------|------------------|-------------------------------------------------|
| `render_plain`  | always available | One line per issue with `file:line:col`, for CI |
| `render_pretty` | `color`          | rustc-style caret output with source excerpts   |
| `render_json`   | `serde`          | Structured JSON for tooling                     |

```rust
let mut out = String::new();
report.render_pretty( & sources, & mut out) ?;
eprint!("{out}");
```

Pretty output underlines the offending value in its source line:

```
error: unknown load_balancing_strategy: failovr
  --> ingress.d/api.hcl:12:31
   |
12 |   load_balancing_strategy = "failovr"
   |                             ^^^^^^^^^
   = help: expected one of: failover, round_robin, request_pressure, sticky_hash, random
```

Line and column lookups are O(log n) via a per-source line index. Columns count characters, not bytes.

## The hcl module

The `hcl` feature provides an adapter over `hcl-edit` for span-faithful parsing.

### FromHcl

Types that parse from HCL implement:

```rust
pub trait FromHcl: Sized {
    fn from_hcl(fields: &Fields<'_>, report: &mut Report) -> Option<Self>;
}
```

Returning `None` means the value could not be constructed; the reason is already in the report.
Implementations should parse every field they can before returning, so one bad field does not hide
the others.

### Fields

`Fields` is a normalized view over the members of an HCL container. It is the reason both HCL
spellings work everywhere:

```hcl
bind {
  # block syntax
  port = 8080
}

bind = {
  # attribute-with-object syntax
  port = 8080
}
```

`Fields::of_body` walks a block body; `Fields::of_object` walks an object expression. Both produce
the same `Field` records, so a `FromHcl` impl never needs to know which spelling the operator used.
Each `Field` carries its name, the span of the name, the span of the whole entry, and a `FieldKind`:
`Value(&Expression)` for attributes or `Block(&Block)` for nested blocks.

### Parsing helpers

Leaf parsers convert one `Field` into a `Located` value, reporting a typed error on mismatch:

- `parse_string_field`, `parse_int_field` (i64), `parse_float_field`, `parse_bool_field`
- `parse_string_list_field` for arrays of strings

Structural parsers recurse through `FromHcl`:

- `parse_struct_field`: one nested struct from a block or object value
- `parse_single_struct`: like `parse_struct_field`, but reports duplicates when the block appears
  more than once
- `parse_struct_list_field`: repeated blocks or an array of objects, collected into a `Vec`

Reporting helpers keep messages uniform: `report_unknown_field`, `report_missing_field`,
`report_duplicate_field`. Unknown fields are always errors; there is no lenient mode.

`parse_hcl<T: FromHcl>(&sources, id, &mut report)` is the top-level entry point: it parses the
registered source text into an HCL tree and runs `T::from_hcl` over it. `span_of` converts any
`hcl_edit` node span into a confval `Span`.

:::note
`hcl-edit` rejects duplicate attribute keys at parse time, so duplicate attributes surface as syntax
errors. Duplicate blocks parse fine and are reported by `parse_single_struct` with a related span
pointing at the first occurrence.
:::

## Derive macros

The `derive` feature (which implies `hcl`) provides two macros from the `confval-derive` crate.

### #[derive(confval::Spec)]

Generates the `FromHcl` impl for a spec struct. Parsing is purely structural; the macro never embeds
semantic rules.

```rust
#[derive(confval::Spec)]
pub struct ServerSpec {
    pub version: Located<HclInt>,
    pub threads: Option<Located<HclInt>>,

    #[confval(nested)]
    pub tls_automation: Option<Located<TlsAutomationSpec>>,

    #[confval(default = 30)]
    pub dns_refresh_interval_seconds: Located<HclInt>,
}
```

Field rules:

- Leaf fields dispatch by type to the matching parser: `Located<String>`, `Located<i64>` /
  `Located<HclInt>`, `Located<f64>`, `Located<bool>`, `Located<PathBuf>`, `Vec<Located<String>>`,
  and `Option<Located<Vec<Located<String>>>>`.
- `Option<...>` makes a field optional. A non-optional field with no default reports
  `missing field` when absent.
- **`#[confval(nested)]`** delegates to the field type's own `FromHcl` impl. Works for single
  structs, optional structs, and `Vec` of structs (repeated blocks).
- **`#[confval(default)]`** / **`#[confval(default = expr)]`** fills an absent field with a detached
  default instead of reporting it missing.
- Unknown fields in the input are reported as errors.

Tagged unions (a block whose shape depends on a discriminator field like `mode` or `type`) are
hand-written `FromHcl` impls; the derive only handles plain structs.

### #[derive(confval::Config)]

Generates the `Lower` impl that converts a validated spec into a runtime config:

```rust
#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
pub struct ServerConfig {
    #[confval(lower(from = version, with = i64_to_u32))]
    pub version: u32,

    #[confval(nested)]
    pub tls_automation: Option<TlsAutomationConfig>,

    pub ca_file: Option<String>,
}
```

The `Lower` trait is:

```rust
pub trait Lower<S>: Sized {
    fn lower(spec: &S, report: &mut Report) -> Option<Self>;
}
```

Field rules:

- **No attribute**: the field auto-maps via the `LowerAuto` trait, which strips `Located` wrappers
  without narrowing: `Located<T> -> T`, `Option<Located<T>> -> Option<T>`,
  `Vec<Located<T>> -> Vec<T>`, `Located<Vec<Located<T>>> -> Vec<T>`, and the optional variant of
  the last.
- **`#[confval(nested)]`**: the field type implements `Lower` itself; works for single, `Option`,
  and `Vec` shapes.
- **`#[confval(lower(from = field, with = fn))]`**: explicit conversion through a function
  `fn(&SpecField, &mut Report) -> Option<Target>`. All narrowing (`i64` to `u16`, string to enum,
  string to `IpNet`) goes through these functions. `from` also accepts a tuple `(a, b)` when one
  config field derives from several spec fields.
- **`#[confval(spec_only(field, ...))]`** at the struct level names spec fields that intentionally
  have no runtime counterpart.

The generated impl destructures the spec exhaustively with no rest pattern. Adding a field to either
struct without accounting for it on the other side is a compile error, which keeps spec and config
in lockstep.

For integer width changes, `confval::provenance::narrow` provides ready-made `with` functions:
`i64_to_u16`, `i64_to_u32`, `i64_to_u64`, `i64_to_usize`, and `opt_` variants for optional fields.
They narrow with `try_from` rather than `as`: a value that does not fit is reported at its span and
lowering fails, so a missing range rule surfaces as a located error instead of a silent truncation.

```rust
use confval::provenance::narrow;

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(lower(from = port, with = narrow::i64_to_u16))]
    port: u16,
}
```

## RangeConstraint

Numeric bounds are declared once and checked against located values:

```rust
range_constraint!(PORT, i64, min: 1, max: 65535);
range_constraint!(DRAIN, i64, min: 0, max: 300, units: "seconds");
range_constraint!(WORKERS, i64, min: 1, max: 512, help: "Match this to your CPU core count.");

PORT.check_located( & spec.port, "port", report);
```

`check_located` emits an error at the value's span when out of range. When **help** is provided it
overrides the auto-generated suggestion; otherwise confval generates one like "Set port to at
least 1".

## Validate

`Validate` is field-local semantic validation for a spec type:

```rust
pub trait Validate {
    fn validate(&self, report: &mut Report);
}
```

A `Validate` impl checks what a value can prove about itself from its own fields, reporting at the
span each field already carries. It takes only `&self` and the report: no span and no origin
parameter, because anything needing more context (a missing required child, a cross-field rule, a
relational check across the whole config) belongs in the consumer's central validators, not here.

The trait's reason to exist is to be nameable in a bound. The `Config` derive, given the `validate`
flag, emits it on the generated `Lower` impl:

```rust
#[derive(confval::Config)]
#[confval(lower_from = ServerSpec, validate)]
struct ServerConfig { /* ... */ }
// generates: impl Lower<ServerSpec> for ServerConfig where ServerSpec: Validate { ... }
```

A flagged config whose spec has no `Validate` impl then fails to compile, so a spec that can be
lowered into a runtime config but carries no validator is unrepresentable. The flag is opt-in:
configs that do not request it lower exactly as before. Hand-written `Lower` impls add the same
`where S: Validate` clause directly, and a flattening lowering (one that has no per-entity `Lower`
impl) can put the bound on the function that performs it.

The bound guarantees the validator exists, not that lowering calls it. Validation is still invoked
explicitly before the gate; the trait closes the "forgot to write a validator" gap, the call site
remains the consumer's responsibility.

## Feature flags

| Flag     | Default | Brings in        | Enables                                                  |
|----------|---------|------------------|----------------------------------------------------------|
| `serde`  | off     | `serde`          | `Located` serde impls, `render_json`                     |
| `color`  | off     | `owo-colors`     | `render_pretty` with ANSI color                          |
| `hcl`    | off     | `hcl-edit`       | The `confval::hcl` adapter module                        |
| `derive` | off     | `confval-derive` | `#[derive(Spec)]` and `#[derive(Config)]`; implies `hcl` |

Snakeway enables all four in its workspace dependency.

## The pipeline contract

confval assumes a fixed phase ordering, and the derives are designed around it:

1. **Parse** (structural): `parse_hcl` builds specs and reports shape problems.
2. **Validate** (semantic): `Validate` impls take `&self` and `&mut Report` and check ranges, closed
   sets, and cross-field rules against the spans stored in `Located` fields. The trait doubles as a
   compile-time bound on step 4, so a lowerable spec without a validator does not compile.
3. **Gate**: lowering must not run on a report that contains errors.
4. **Lower**: `Lower::lower` converts specs to runtime types. Because the gate ran, narrowing
   conversions in `with` functions are safe.

A complete end-to-end example lives in `crates/confval/examples/basic.rs`. See
[Configuration Internals](/internals/configuration) for how Snakeway applies this pipeline.
