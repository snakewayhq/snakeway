# confval-derive

Derive macros for [`confval`](https://crates.io/crates/confval).

This crate provides two procedural macros that remove the boilerplate from a
span-first config pipeline:

- `#[derive(Spec)]` writes the code that **parses** a struct out of a config
  file (an `impl confval::format::FromFields`).
- `#[derive(Config)]` writes the code that **lowers** a parsed spec into the
  runtime form your program actually uses (an `impl confval::pipeline::Lower`).

## Usage

You normally don't depend on `confval-derive` directly. Enable the `derive`
feature on `confval`, which re-exports both macros as `confval::Spec` and
`confval::Config`:

```toml
[dependencies]
confval = { version = "0.2", features = ["derive"] }
```

```rust
use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    hostname: Located<String>,
    port: Located<i64>,
    #[confval(default = 4)]
    workers: Located<i64>,
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    hostname: String,
    #[confval(lower(from = port, with = port_to_u16))]
    port: u16,
    #[confval(lower(from = workers, with = workers_to_usize))]
    workers: usize,
}
```

See the runnable [`hcl`](https://github.com/snakewayhq/snakeway/blob/main/crates/confval/examples/hcl.rs)
and [`toml`](https://github.com/snakewayhq/snakeway/blob/main/crates/confval/examples/toml.rs)
examples in the `confval` crate for the full parse → validate → lower → render flow.

## Attribute reference

Both macros read `#[confval(...)]` attributes. A field with no attribute is
mapped automatically from the same-named spec field.

### `#[derive(Spec)]`

| Attribute                    | Applies to                                                 | Effect                                                 |
|------------------------------|------------------------------------------------------------|--------------------------------------------------------|
| `#[confval(nested)]`         | a `Located<S>` or `Option<Located<S>>` field               | parse the inner struct (which must also derive `Spec`) |
| `#[confval(default)]`        | a leaf, a bare string list, or a non-optional nested field | fill an absent value with its type's `Default`         |
| `#[confval(default = expr)]` | a leaf field                                               | fill an absent value with `expr`                       |

### `#[derive(Config)]`

Struct-level:

| Attribute                           | Effect                                                                                |
|-------------------------------------|---------------------------------------------------------------------------------------|
| `#[confval(lower_from = SpecType)]` | the spec this config lowers from (required)                                           |
| `#[confval(spec_only(a, b))]`       | spec fields with no config counterpart, ignored during lowering                       |
| `#[confval(validate)]`              | bind the spec to `Validate`, so a lowerable spec without a validator fails to compile |

Field-level:

| Attribute                                      | Effect                                                                                                                          |
|------------------------------------------------|---------------------------------------------------------------------------------------------------------------------------------|
| *(none)*                                       | copy from the same-named spec field, unwrapping `Located`                                                                       |
| `#[confval(nested)]`                           | lower the same-named spec field through its own `Lower` impl                                                                    |
| `#[confval(nested, default)]`                  | lower a non-optional config field from an `Option<Located<S>>` spec field, substituting `S::default()` when the block is absent |
| `#[confval(lower(from = field, with = func))]` | build this field by calling `func` on the named spec field(s); `from` accepts a single name or a tuple                          |

The generated lowering destructures the spec exhaustively, so a spec field that
no config field consumes is a compile error rather than a silent drop.

## Documentation

See the [confval documentation](https://snakeway.dev/docs/internals/confval) for
the full API overview and the design of the span-first pipeline.

## License

Apache-2.0
