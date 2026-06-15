use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    hosts: Located<Vec<Located<String>>>,
}

fn main() {}
