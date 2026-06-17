use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<u16>,
}

fn main() {}
