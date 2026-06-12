use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    #[confval(rename = "port")]
    port: Located<i64>,
}

fn main() {}
