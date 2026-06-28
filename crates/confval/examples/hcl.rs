//! End-to-end example: parse an HCL config span-first, validate it, lower
//! it to a runtime type, and render the diagnostics.
//!
//! The companion `toml` example defines the same `ServerSpec`/`ServerConfig`
//! and differs only in the source text and the one-line parse call, showing
//! that everything after parsing is format-neutral.
//!
//! Beyond the flat fields, this pair exercises a nested block that is optional
//! in the source but defaulted at runtime (`limits`), a `KeywordSet`-validated
//! keyword field (`mode`), and the ready-made `narrow` helpers alongside a
//! hand-written `with` function.
//!
//! Run with: cargo run -p confval --example hcl --features derive,color,hcl

use confval::{
    KeywordSet, RangeConstraint,
    diagnostic::Report,
    pipeline::{Lower, narrow},
    range_constraint,
    source::{Located, SourceMap},
};

range_constraint!(PORT, i64, min: 1, max: 65535);
range_constraint!(WORKERS, i64, min: 1, max: 512);
range_constraint!(MAX_BODY_MB, i64, min: 1, max: 1024);

const LIMIT_MODES: [&str; 3] = ["enforce", "log", "off"];

#[derive(confval::Spec)]
struct ServerSpec {
    hostname: Located<String>,
    port: Located<i64>,
    #[confval(default = 4)]
    workers: Located<i64>,
    // Optional in the source: when the block is omitted the spec keeps it
    // `None`, so a spec dump stays source-faithful. The config side fills the
    // default at lowering time.
    #[confval(nested)]
    limits: Option<Located<LimitsSpec>>,
}

#[derive(confval::Spec)]
struct LimitsSpec {
    #[confval(default = 16)]
    max_body_mb: Located<i64>,
    #[confval(default = "enforce".to_string())]
    mode: Located<String>,
}

impl Default for LimitsSpec {
    fn default() -> Self {
        Self {
            max_body_mb: Located::detached(16),
            mode: Located::detached("enforce".to_string()),
        }
    }
}

fn validate_server(spec: &ServerSpec, report: &mut Report) {
    PORT.check_located(&spec.port, "port", report);
    WORKERS.check_located(&spec.workers, "workers", report);

    if let Some(limits) = &spec.limits {
        MAX_BODY_MB.check_located(&limits.value.max_body_mb, "max_body_mb", report);
        KeywordSet::new(&LIMIT_MODES).check_located(&limits.value.mode, "mode", report);
    }

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
    #[confval(lower(from = port, with = narrow::i64_to_u16))]
    port: u16,
    #[confval(lower(from = workers, with = workers_to_usize))]
    workers: usize,
    // The spec field is `Option<Located<LimitsSpec>>`; with `default` an absent
    // block lowers `LimitsSpec::default()` instead of producing a missing-field
    // error, and the runtime field stays non-optional.
    #[confval(nested, default)]
    limits: LimitsConfig,
}

#[derive(confval::Config)]
#[confval(lower_from = LimitsSpec)]
struct LimitsConfig {
    #[confval(lower(from = max_body_mb, with = narrow::i64_to_u16))]
    max_body_mb: u16,
    mode: String,
}

fn workers_to_usize(value: &Located<i64>, _report: &mut Report) -> Option<usize> {
    // Safe: the range was validated and lowering only runs on a clean report.
    Some(value.value as usize)
}

fn main() {
    let input = r#"hostname = ""
port = 99999

limits {
  mode = "yolo"
}
"#;

    let mut sources = SourceMap::new();
    let mut report = Report::new();
    let id = sources.add("server.hcl", input);

    let spec: Option<ServerSpec> = confval::format::hcl::parse_hcl(&sources, id, &mut report);
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
    println!(
        "limits: max_body_mb={} mode={}",
        config.limits.max_body_mb, config.limits.mode
    );
}
