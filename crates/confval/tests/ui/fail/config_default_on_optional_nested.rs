use confval::source::Located;

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

#[derive(confval::Config)]
#[confval(lower_from = InnerSpec)]
struct InnerConfig {
    name: String,
}

#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(nested)]
    inner: Option<Located<InnerSpec>>,
}

// `#[confval(nested, default)]` fills an absent block with the spec's
// `Default`, so it only makes sense on a non-optional config field. On an
// optional config field an absent block already lowers to `None`, so the
// derive rejects the combination at compile time.
#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(nested, default)]
    inner: Option<InnerConfig>,
}

fn main() {}
