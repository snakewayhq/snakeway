use confval::prelude::{Located, Lower, Report};

#[derive(confval::Spec)]
struct InnerSpec {
    name: Located<String>,
}

#[derive(confval::Spec)]
struct ServerSpec {
    port: Located<i64>,
    #[confval(default = 4)]
    workers: Located<i64>,
    #[confval(nested)]
    inner: Option<Located<InnerSpec>>,
}

#[derive(confval::Config)]
#[confval(lower_from = InnerSpec)]
struct InnerConfig {
    name: String,
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(lower(from = port, with = confval::pipeline::narrow::i64_to_u16))]
    port: u16,
    workers: i64,
    #[confval(nested)]
    inner: Option<InnerConfig>,
}

fn main() {
    let spec = ServerSpec {
        port: Located::detached(8080),
        workers: Located::detached(4),
        inner: None,
    };
    let mut report = Report::new();
    let config = ServerConfig::lower(&spec, &mut report).unwrap();
    assert_eq!(config.port, 8080);
}
