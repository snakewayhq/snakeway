use confval::source::Located;

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(nested)]
    inner: Option<Vec<Located<InnerSpec>>>,
}

fn main() {}
