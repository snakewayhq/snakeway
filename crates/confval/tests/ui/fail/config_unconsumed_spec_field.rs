use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
    threads: Located<i64>,
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    port: i64,
}

fn main() {}
