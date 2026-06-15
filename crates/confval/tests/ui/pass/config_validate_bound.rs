use confval::diagnostic::Report;
use confval::pipeline::Validate;
use confval::source::Located;

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
}

impl Validate for ServerSpec {
    fn validate(&self, _report: &mut Report) {}
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec, validate)]
struct ServerConfig {
    port: i64,
}

fn main() {
    let spec = ServerSpec {
        port: Located::detached(8080),
    };
    let mut report = Report::new();
    let _ = <ServerConfig as confval::provenance::Lower<ServerSpec>>::lower(&spec, &mut report);
}
