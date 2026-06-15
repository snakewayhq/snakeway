use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
}

fn port_to_u16(_value: &Located<i64>, _report: &mut confval::diagnostic::Report) -> Option<u16> {
    None
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(lower(with = port_to_u16))]
    port: u16,
}

fn main() {}
