use confval::source::Located;

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

// A bare `#[confval(default)]` is honored on a non-optional nested field
// (`Located<InnerSpec>`), where it fills an absent block with `InnerSpec::default()`.
// On an *optional* nested field it is meaningless, since an absent block is
// already `None`, so the derive rejects it at compile time.
#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(nested, default)]
    inner: Option<Located<InnerSpec>>,
}

fn main() {}
