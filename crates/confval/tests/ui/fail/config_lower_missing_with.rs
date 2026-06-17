use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(lower(from = port))]
    port: u16,
}

fn main() {}
