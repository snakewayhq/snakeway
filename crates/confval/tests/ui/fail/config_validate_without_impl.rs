use confval::provenance::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
}

// No `impl Validate for ServerSpec`, but the config opts into the bound.

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec, validate)]
struct ServerConfig {
    port: i64,
}

fn main() {}
