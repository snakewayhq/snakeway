use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<u16>,
}

fn main() {}
