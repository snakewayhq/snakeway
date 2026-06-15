//! End-to-end example: parse a TOML config span-first, validate it, lower it
//! to a runtime type, and render the diagnostics.
//!
//! This is the companion to the `hcl` example. The `ServerSpec`,
//! `ServerConfig`, validation, and lowering are identical; only the source
//! text and the single `parse_toml` call differ. Everything after parsing —
//! the derive-generated walk, the validators, the lowering functions — is
//! format-neutral and shared verbatim. Where the `hcl` example feeds an
//! invalid config to show the diagnostics, this one feeds a valid config to
//! show the lowered output.
//!
//! Run with: cargo run -p confval --example toml --features derive,color,toml

use confval::{
    RangeConstraint,
    diagnostic::Report,
    pipeline::Lower,
    range_constraint,
    source::{Located, SourceMap},
};

range_constraint!(PORT, i64, min: 1, max: 65535);
range_constraint!(WORKERS, i64, min: 1, max: 512);

#[derive(confval::Spec)]
struct ServerSpec {
    hostname: Located<String>,
    port: Located<i64>,
    #[confval(default = 4)]
    workers: Located<i64>,
}

fn validate_server(spec: &ServerSpec, report: &mut Report) {
    PORT.check_located(&spec.port, "port", report);
    WORKERS.check_located(&spec.workers, "workers", report);

    if spec.hostname.value.is_empty() {
        report
            .error("hostname must not be empty")
            .at(spec.hostname.span)
            .help("Set hostname to a reachable address, e.g. \"127.0.0.1\".")
            .emit();
    }
}

#[derive(confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    hostname: String,
    #[confval(lower(from = port, with = port_to_u16))]
    port: u16,
    #[confval(lower(from = workers, with = workers_to_usize))]
    workers: usize,
}

fn port_to_u16(value: &Located<i64>, _report: &mut Report) -> Option<u16> {
    // Safe: the range was validated and lowering only runs on a clean report.
    Some(value.value as u16)
}

fn workers_to_usize(value: &Located<i64>, _report: &mut Report) -> Option<usize> {
    Some(value.value as usize)
}

fn main() {
    let input = r#"hostname = "127.0.0.1"
port = 8080
workers = 8
"#;

    let mut sources = SourceMap::new();
    let mut report = Report::new();
    let id = sources.add("server.toml", input);

    // The only format-specific line in the whole program.
    let spec: Option<ServerSpec> = confval::format::toml::parse_toml(&sources, id, &mut report);
    if let Some(spec) = &spec {
        validate_server(spec, &mut report);
    }

    if report.has_errors() {
        let mut out = String::new();
        report.render_pretty(&sources, &mut out).unwrap();
        eprint!("{out}");
        std::process::exit(1);
    }

    let spec = spec.expect("parse returned None without reporting an error");
    let config = ServerConfig::lower(&spec, &mut report).expect("validated config lowers");
    println!(
        "listening on {}:{} with {} workers",
        config.hostname, config.port, config.workers
    );
}
