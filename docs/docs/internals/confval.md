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
- **Format-neutral core.** Parsing produces a format-neutral field model; only a thin frontend knows
  any concrete syntax. HCL and TOML ship today, each behind its own feature, and a new format is one
  more frontend over the same primitives.
- **Minimal dependencies.** The core has none. `serde`, `owo-colors`, `hcl-edit`, `toml_edit`, and the
  derive macros are all behind feature flags.

## Usage Examples

Snakeway has a very complex configuration surface.
It may be overwhelming.
This documentation page is also very dense.

### Crate Examples

The confval crate has a few examples that are easier to understand.

Run the TOML example:

```shell
cargo run -p confval --example toml --features="derive color toml"
```

Run the HCL example:

```shell
cargo run -p confval --example hcl --features="derive color hcl"
```

### Mini-Redis Example

This [demo PR for the mini-redis](https://github.com/ethanhann/mini-redis/pull/1) Tokio example server is a bit more
complex than the crate examples, but not nearly as complex as Snakeway.
It is meant as a stepping stone.

## Crate layout

confval is organized into four layers, each a module, plus a prelude. The dependency direction is
strictly downward: `format` builds on `pipeline`, which builds on `diagnostic`, which builds on
`source`.

| Module                | Holds                                                                                       |
|-----------------------|---------------------------------------------------------------------------------------------|
| `confval::source`     | `Located`, `Span`, `SourceId`, `Source`, `SourceMap` (the "where")                          |
| `confval::diagnostic` | `Report`, `Issue`, `IssueBuilder`, `Severity`, the renderers (the "what")                   |
| `confval::pipeline`   | `Lower`, `LowerAuto`, `Validate`, `narrow`, `RangeConstraint`, `KeywordSet` (the transform) |
| `confval::format`     | the neutral field model (`field`) and the frontends (`hcl`, `toml`)                         |
| `confval::prelude`    | a glob re-export of the common imports across those layers                                  |

`use confval::prelude::*;` pulls the everyday names (`Located`, `Span`, `Report`, `Lower`, `Validate`,
`narrow`, `RangeConstraint`, `KeywordSet`, and the derives) in one line; the explicit module paths
remain available when you want them.

## The source module

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

## The diagnostic module

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

Query methods: `has_errors()`, `has_warnings()`, `has_issues()`. Severity is the two-variant
`Severity` enum (`Error`, `Warning`).

### Rendering

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
The data model (`Report`, `Issue`) and the renderers live in separate files in the module; an issue
records only a severity, message, optional span, optional help, and related spans, and never touches
source text until render time.

## The format layer

`confval::format` is where text becomes data. It has two halves: a format-neutral field model that
everything downstream works against, and one frontend per syntax that produces it.

### The neutral field model

A frontend parses its own syntax tree and lowers it into the owned types in `confval::format::field`
(re-exported at `confval::format`). Once a `Fields` exists, nothing downstream knows which format it
came from.

- **`Fields`** is one structural level: the named entries of a body, a table, or an inline object,
  plus the enclosing span a missing-field error points at.
- **`Field`** is one entry: its name, the span of the name, the span of the whole entry, and a
  `FieldKind`.
- **`FieldKind`** is `Value(Value)` for an attribute (`name = value`) or `Block(Fields)` for a block
  (`name { ... }` in HCL, `[name]` in TOML). The split exists so diagnostics can say "found block"
  rather than "found object".
- **`Value`** is a span plus a `ValueKind`: `Scalar(Scalar)`, `Seq(Vec<Value>)`, `Map(Fields)`, or
  `Other(&'static str)`.
- **`Scalar`** is `String`, `Int(i64)`, `Float(f64)`, or `Bool`. Integers and floats are kept distinct
  so a format that separates them syntactically (TOML's `1` vs `1.0`) round-trips faithfully.
- **`Other(label)`** is a value present in source but outside the model (an HCL template or `null`, a
  TOML datetime). No leaf parser matches it, so it always surfaces as an ordinary type mismatch whose
  noun is the label, for example `expected string, found datetime`.

### FromFields

Types that parse from configuration implement the format-neutral trait:

```rust
pub trait FromFields: Sized {
    fn from_fields(fields: &Fields, report: &mut Report) -> Option<Self>;
}
```

Returning `None` means the value could not be constructed; the reason is already in the report.
Implementations should parse every field they can before returning, so one bad field does not hide
the others.

### Parsing helpers

Leaf parsers convert one `Field` into a `Located` value, reporting a typed error on mismatch:

- `parse_string_field`, `parse_int_field` (i64), `parse_float_field`, `parse_bool_field`
- `parse_string_list_field` for arrays of strings

Structural parsers recurse through `FromFields`:

- `parse_struct_field`: one nested struct from a block or a map value
- `parse_single_struct`: like `parse_struct_field`, but reports duplicates when the field appears more
  than once
- `parse_struct_list_field`: repeated blocks or a sequence of maps, collected into a `Vec`

Reporting helpers keep messages uniform: `report_unknown_field`, `report_missing_field`,
`report_duplicate_field`. Unknown fields are always errors; there is no lenient mode. All of these are
format-neutral and shared by every frontend.

### Frontends

A frontend's only job is text-to-`Fields`. Each is a thin module behind its own feature:

| Entry point                         | Feature | Parser      |
|-------------------------------------|---------|-------------|
| `confval::format::hcl::parse_hcl`   | `hcl`   | `hcl-edit`  |
| `confval::format::toml::parse_toml` | `toml`  | `toml_edit` |

Both have the signature `fn(&SourceMap, SourceId, &mut Report) -> Option<T> where T: FromFields`, and
both produce the same neutral `Fields`, so the leaf parsers, the derive output, and every hand-written
`FromFields` impl work against either format unchanged.

The two HCL spellings normalize the same way: a block (`bind { port = 8080 }`) becomes a
`FieldKind::Block`, and an attribute-with-object (`bind = { port = 8080 }`) becomes a
`FieldKind::Value` holding a `Map`. TOML maps analogously: a `[table]` is a block, an inline `{ ... }`
is a map value, and an array of tables (`[[x]]`) becomes one field whose value is a sequence of maps,
so a `Vec` of nested structs lowers from it exactly as it would from an HCL array of objects.

:::note
`hcl-edit` rejects duplicate attribute keys at parse time, so duplicate attributes surface as syntax
errors. Duplicate blocks parse fine and are reported by `parse_single_struct` with a related span
pointing at the first occurrence.
:::

## Derive macros

The `derive` feature provides two macros from the `confval-derive` crate. The macros emit format-neutral
code, so `derive` no longer implies any frontend; pair it with `hcl` and/or `toml`.

### #[derive(confval::Spec)]

Generates the `FromFields` impl for a spec struct. Parsing is purely structural; the macro never embeds
semantic rules.

```rust
#[derive(confval::Spec)]
pub struct ServerSpec {
    pub version: Located<i64>,
    pub threads: Option<Located<i64>>,

    #[confval(nested)]
    pub tls_automation: Option<Located<TlsAutomationSpec>>,

    #[confval(default = 30)]
    pub dns_refresh_interval_seconds: Located<i64>,
}
```

Field rules:

- Leaf fields dispatch by type to the matching parser: `Located<String>`, `Located<i64>`,
  `Located<f64>`, `Located<bool>`, `Located<PathBuf>`, `Vec<Located<String>>`, and
  `Option<Located<Vec<Located<String>>>>`.
- `Option<...>` makes a field optional. A non-optional field with no default reports
  `missing field` when absent.
- **`#[confval(nested)]`** delegates to the field type's own `FromFields` impl. Works for single
  structs, optional structs, and `Vec` of structs (repeated blocks).
- **`#[confval(default)]`** / **`#[confval(default = expr)]`** fills an absent field with a detached
  default instead of reporting it missing. A bare `#[confval(default)]` also applies to a non-optional
  nested field (`Located<S>` with `#[confval(nested, default)]`), filling an omitted block with
  `S::default()`; `default = expr` is leaf-only.
- Unknown fields in the input are reported as errors.

Tagged unions (a block whose shape depends on a discriminator field like `mode` or `type`) are
hand-written `FromFields` impls; the derive only handles plain structs.

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
- **`#[confval(nested, default)]`**: a non-optional config field lowered from an `Option<Located<S>>`
  spec field. When the source omits the block, `S::default()` is lowered in its place, so the runtime
  field is always populated while the spec stays source-faithful (an absent block stays `None`). This
  is the leaf-default's structural counterpart, and replaces hand-written `*_or_default` lowering
  functions.
- **`#[confval(lower(from = field, with = fn))]`**: explicit conversion through a function
  `fn(&SpecField, &mut Report) -> Option<Target>`. All narrowing (`i64` to `u16`, string to enum,
  string to `IpNet`) goes through these functions. `from` also accepts a tuple `(a, b)` when one
  config field derives from several spec fields.
- **`#[confval(spec_only(field, ...))]`** at the struct level names spec fields that intentionally
  have no runtime counterpart.

The generated impl destructures the spec exhaustively with no rest pattern. Adding a field to either
struct without accounting for it on the other side is a compile error, which keeps spec and config
in lockstep.

`confval::pipeline::narrow` provides ready-made `with` functions. For integer width changes:
`i64_to_u16`, `i64_to_u32`, `i64_to_u64`, `i64_to_usize`, and `opt_` variants for optional fields.
They narrow with `try_from` rather than `as`: a value that does not fit is reported at its span and
lowering fails, so a missing range rule surfaces as a located error instead of a silent truncation.
`i64_secs_to_duration` (and `opt_i64_secs_to_duration`) route a seconds count through the same checked
narrow into a `Duration`, rejecting a negative value rather than wrapping it; `i64_to_f64` widens to
`f64` for the ratio and rate fields where an `as` cast cannot be named in a `with` attribute.

```rust
use confval::pipeline::narrow;

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

## KeywordSet

Closed sets of allowed keyword strings are checked against located values, the string counterpart of
`RangeConstraint` for the spec layer's load-balancing strategies, log levels, fail policies, and the
like:

```rust
const LOAD_BALANCING_STRATEGIES: [&str; 5] =
    ["failover", "round_robin", "request_pressure", "sticky_hash", "random"];

KeywordSet::new( & LOAD_BALANCING_STRATEGIES)
.check_located( & spec.load_balancing_strategy, "load_balancing_strategy", report);
```

`check_located` reports `unknown {field}: {value}` at the value's span, with a help line of
`expected one of: <comma-joined options>` (the diagnostic shown in the rendering example above). Every
keyword field reports the same way, so the operator sees a consistent message and the allowed set
wherever a closed-set field is wrong.

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
struct ServerConfig {
    /* ... */
}
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

| Flag     | Default | Brings in        | Enables                                                    |
|----------|---------|------------------|------------------------------------------------------------|
| `serde`  | off     | `serde`          | `Located` serde impls, `render_json`                       |
| `color`  | off     | `owo-colors`     | `render_pretty` with ANSI color                            |
| `hcl`    | off     | `hcl-edit`       | The `confval::format::hcl` frontend                        |
| `toml`   | off     | `toml_edit`      | The `confval::format::toml` frontend                       |
| `derive` | off     | `confval-derive` | `#[derive(Spec)]` and `#[derive(Config)]` (format-neutral) |

Frontends are independent opt-ins. The derive emits the format-neutral `FromFields`, so `derive`
brings in no parser on its own; pick `hcl` and/or `toml` for the format you actually read. Snakeway
enables `serde`, `color`, `hcl`, and `derive` in its workspace dependency.

## The pipeline contract

confval assumes a fixed phase ordering, and the derives are designed around it:

1. **Parse** (structural): a frontend (`parse_hcl` or `parse_toml`) builds the neutral `Fields`, runs
   `FromFields`, and reports shape problems.
2. **Validate** (semantic): `Validate` impls take `&self` and `&mut Report` and check ranges, closed
   sets, and cross-field rules against the spans stored in `Located` fields. The trait doubles as a
   compile-time bound on step 4, so a lowerable spec without a validator does not compile.
3. **Gate**: lowering must not run on a report that contains errors.
4. **Lower**: `Lower::lower` converts specs to runtime types. Because the gate ran, narrowing
   conversions in `with` functions are safe.

Two runnable end-to-end examples live in `crates/confval/examples/`: `hcl.rs` and `toml.rs` define the
same `ServerSpec`/`ServerConfig` and differ only in the source text and the single `parse_hcl` vs
`parse_toml` call, demonstrating that everything after parsing is format-neutral. See
[Configuration Internals](/internals/configuration) for how Snakeway applies this pipeline.
