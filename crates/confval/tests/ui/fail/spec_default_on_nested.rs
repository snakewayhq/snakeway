use confval::source::Located;

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

// `#[confval(default)]` is only honored on leaf fields. On a nested field it
// would be silently ignored, so the derive rejects it at compile time.
#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(nested, default)]
    inner: Option<Located<InnerSpec>>,
}

fn main() {}
