#[derive(confval::Config)]
#[confval(lowered_from = ServerSpec)]
struct ServerConfig {
    port: u16,
}

fn main() {}
