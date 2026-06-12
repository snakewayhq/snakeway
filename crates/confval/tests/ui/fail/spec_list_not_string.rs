use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    ports: Located<Vec<Located<i64>>>,
}

fn main() {}
