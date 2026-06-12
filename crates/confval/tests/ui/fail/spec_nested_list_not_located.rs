use confval::provenance::Located;

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(nested)]
    inner: Vec<InnerSpec>,
}

fn main() {}
