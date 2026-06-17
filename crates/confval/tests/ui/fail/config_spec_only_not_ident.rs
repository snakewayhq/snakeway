use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec, spec_only(some::path))]
struct ServerConfig {
    port: i64,
}

fn main() {}
