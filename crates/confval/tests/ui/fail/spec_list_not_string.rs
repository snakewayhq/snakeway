use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    ports: Located<Vec<Located<i64>>>,
}

fn main() {}
