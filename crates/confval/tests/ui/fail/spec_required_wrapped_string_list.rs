use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    hosts: Located<Vec<Located<String>>>,
}

fn main() {}
