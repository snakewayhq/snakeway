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
//! The `limits` block is omitted here, so the output shows the config-side
//! `#[confval(nested, default)]` materializing `LimitsSpec::default()` at
//! runtime while the spec stays source-faithful.
//!
//! Run with: cargo run -p confval --example toml --features derive,color,toml

use confval::prelude::*;

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
    // Optional in the source: when the block is omitted, the spec keeps it
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
    println!(
        "limits: max_body_mb={} mode={}",
        config.limits.max_body_mb, config.limits.mode
    );
}
