//! End-to-end exercise of the span-first pipeline: a hand-written Spec and
//! Config pair driven through parse, validation, the error gate, and
//! lowering. These impls are the reference shape for generated code, so
//! changes here should stay deliberate.

#![cfg(feature = "derive")]

use confval::hcl::parse_hcl;
use confval::provenance::{Located, Lower, Report, SourceMap, Span};
use std::net::SocketAddr;
use std::path::PathBuf;
use std::time::Duration;

#[derive(Debug, confval::Spec)]
struct ServerSpec {
    hostname: Located<String>,
    port: Located<i64>,
    daemon: Located<bool>,
    max_connections: Located<i64>,
    shutdown_timeout_seconds: Located<i64>,
    pid_file: Option<Located<String>>,
    allow: Option<Located<Vec<Located<String>>>>,
    #[confval(nested)]
    tls: Option<Located<TlsSpec>>,
}

#[derive(Debug, confval::Spec)]
struct TlsSpec {
    cert: Located<String>,
    key: Located<String>,
}

fn check_range(value: &Located<i64>, min: i64, max: i64, name: &str, report: &mut Report) {
    if !(min..=max).contains(&value.value) {
        report
            .error(format!("{name} out of range: {}", value.value))
            .at(value.span)
            .help(format!("use a value between {min} and {max}"))
            .emit();
    }
}

/// Entity validation: runs after parsing on the Located values, so spans
/// survive without the parser's involvement.
fn validate_server_spec(spec: &ServerSpec, report: &mut Report) {
    check_range(&spec.port, 1, 65535, "port", report);
    check_range(&spec.max_connections, 1, 250, "max_connections", report);
    check_range(
        &spec.shutdown_timeout_seconds,
        1,
        3600,
        "shutdown_timeout_seconds",
        report,
    );
}

#[derive(Debug, confval::Config)]
#[confval(lower_from = ServerSpec)]
struct ServerConfig {
    #[confval(lower(from = (hostname, port), with = parse_addr))]
    addr: SocketAddr,
    daemon: bool,
    #[confval(lower(from = max_connections, with = max_connections_to_usize))]
    max_connections: usize,
    #[confval(lower(from = shutdown_timeout_seconds, with = seconds_to_duration))]
    shutdown_timeout: Duration,
    #[confval(lower(from = pid_file, with = pid_file_to_path))]
    pid_file: Option<PathBuf>,
    #[confval(lower(from = allow, with = allow_to_vec))]
    allow: Vec<String>,
    #[confval(nested)]
    tls: Option<TlsConfig>,
}

#[derive(Debug, confval::Config)]
#[confval(lower_from = TlsSpec)]
struct TlsConfig {
    #[confval(lower(from = cert, with = string_to_path))]
    cert: PathBuf,
    #[confval(lower(from = key, with = string_to_path))]
    key: PathBuf,
}

fn max_connections_to_usize(value: &Located<i64>, _report: &mut Report) -> Option<usize> {
    // Safe: the range was validated and the caller gates lowering on a
    // clean report.
    Some(value.value as usize)
}

fn seconds_to_duration(value: &Located<i64>, _report: &mut Report) -> Option<Duration> {
    Some(Duration::from_secs(value.value as u64))
}

fn pid_file_to_path(
    value: &Option<Located<String>>,
    _report: &mut Report,
) -> Option<Option<PathBuf>> {
    Some(value.as_ref().map(|p| PathBuf::from(&p.value)))
}

fn allow_to_vec(
    value: &Option<Located<Vec<Located<String>>>>,
    _report: &mut Report,
) -> Option<Vec<String>> {
    Some(
        value
            .as_ref()
            .map(|list| list.value.iter().map(|cidr| cidr.value.clone()).collect())
            .unwrap_or_default(),
    )
}

fn string_to_path(value: &Located<String>, _report: &mut Report) -> Option<PathBuf> {
    Some(PathBuf::from(&value.value))
}

fn parse_addr(
    hostname: &Located<String>,
    port: &Located<i64>,
    report: &mut Report,
) -> Option<SocketAddr> {
    let candidate = format!("{}:{}", hostname.value, port.value);
    match candidate.parse() {
        Ok(addr) => Some(addr),
        Err(_) => {
            report
                .error(format!("invalid address: {candidate}"))
                .at(Span::merge(hostname.span, port.span))
                .emit();
            None
        }
    }
}

/// The composition contract: parse, gate on errors, lower, gate again.
fn load(text: &str) -> (SourceMap, Report, Option<ServerConfig>) {
    let mut sources = SourceMap::new();
    let mut report = Report::new();
    let id = sources.add("server.hcl", text);

    let spec: Option<ServerSpec> = parse_hcl(&sources, id, &mut report);
    if let Some(spec) = &spec {
        validate_server_spec(spec, &mut report);
    }
    if report.has_errors() {
        return (sources, report, None);
    }
    let spec = spec.expect("parse returned None without reporting an error");

    let config = ServerConfig::lower(&spec, &mut report);
    if report.has_errors() {
        return (sources, report, None);
    }
    let config = config.expect("lower returned None without reporting an error");
    (sources, report, Some(config))
}

const VALID: &str = r#"hostname = "127.0.0.1"
port = 8080
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30
allow = ["10.0.0.0/8", "192.168.0.0/16"]

tls {
  cert = "cert.pem"
  key = "key.pem"
}
"#;

#[test]
fn valid_config_parses_and_lowers() {
    let (_, report, config) = load(VALID);
    assert!(!report.has_issues(), "issues: {:?}", report.issues());

    let config = config.unwrap();
    assert_eq!(config.addr, "127.0.0.1:8080".parse().unwrap());
    assert!(!config.daemon);
    assert_eq!(config.max_connections, 100);
    assert_eq!(config.shutdown_timeout, Duration::from_secs(30));
    assert_eq!(config.pid_file, None);
    assert_eq!(config.allow, vec!["10.0.0.0/8", "192.168.0.0/16"]);
    let tls = config.tls.unwrap();
    assert_eq!(tls.cert, PathBuf::from("cert.pem"));
    assert_eq!(tls.key, PathBuf::from("key.pem"));
}

#[test]
fn spec_values_carry_byte_accurate_spans() {
    let mut sources = SourceMap::new();
    let mut report = Report::new();
    let id = sources.add("server.hcl", VALID);

    let spec: ServerSpec = parse_hcl(&sources, id, &mut report).unwrap();
    let span = spec.hostname.span;
    assert_eq!(
        &VALID[span.start as usize..span.end as usize],
        "\"127.0.0.1\""
    );

    let allow = spec.allow.unwrap();
    let first = &allow.value[0];
    assert_eq!(
        &VALID[first.span.start as usize..first.span.end as usize],
        "\"10.0.0.0/8\""
    );

    let tls = spec.tls.unwrap();
    let block = &VALID[tls.span.start as usize..tls.span.end as usize];
    assert!(block.starts_with("tls {"), "got: {block:?}");
    assert!(block.ends_with('}'), "got: {block:?}");
}

#[test]
fn all_problems_are_reported_in_one_pass() {
    // The type mismatch is on an optional field so the tree still builds
    // and validation runs; a mismatch on a required field stops the entity
    // at structural errors.
    let input = r#"hostname = "127.0.0.1"
port = 99999
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30
pid_file = 42
hostnme = "typo"
"#;
    let (_, report, config) = load(input);
    assert!(config.is_none());

    let messages: Vec<&str> = report
        .issues()
        .iter()
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        messages.contains(&"port out of range: 99999"),
        "got: {messages:?}"
    );
    assert!(
        messages.contains(&"expected string, found number"),
        "got: {messages:?}"
    );
    assert!(
        messages.contains(&"unknown field: hostnme"),
        "got: {messages:?}"
    );
    assert_eq!(messages.len(), 3, "got: {messages:?}");
}

#[test]
fn out_of_range_value_is_retained_but_gated_from_lowering() {
    let input = r#"hostname = "127.0.0.1"
port = 99999
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30
"#;
    let mut sources = SourceMap::new();
    let mut report = Report::new();
    let id = sources.add("server.hcl", input);

    let spec: Option<ServerSpec> = parse_hcl(&sources, id, &mut report);
    let spec = spec.expect("out-of-range values still produce the tree");
    assert!(!report.has_errors(), "parsing is structural only");
    validate_server_spec(&spec, &mut report);
    assert_eq!(spec.port.value, 99999);
    assert!(report.has_errors());

    let (_, report, config) = load(input);
    assert!(config.is_none());
    assert!(report.has_errors());
}

#[test]
fn missing_required_fields_are_all_reported() {
    let input = "hostname = \"127.0.0.1\"\ndaemon = true\n";
    let (_, report, config) = load(input);
    assert!(config.is_none());

    let messages: Vec<&str> = report
        .issues()
        .iter()
        .map(|issue| issue.message.as_str())
        .collect();
    assert!(
        messages.contains(&"missing required field: port"),
        "got: {messages:?}"
    );
    assert!(
        messages.contains(&"missing required field: max_connections"),
        "got: {messages:?}"
    );
    assert!(
        messages.contains(&"missing required field: shutdown_timeout_seconds"),
        "got: {messages:?}"
    );
}

#[test]
fn missing_field_in_nested_block_is_reported() {
    let input = r#"hostname = "127.0.0.1"
port = 8080
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30

tls {
  cert = "cert.pem"
}
"#;
    let (_, report, config) = load(input);
    assert!(config.is_none());
    let issue = report
        .issues()
        .iter()
        .find(|issue| issue.message == "missing required field: key")
        .expect("missing key in tls block must be reported");
    assert!(
        issue.span.is_some(),
        "missing-field errors carry the enclosing span"
    );
}

#[test]
fn duplicate_attribute_is_a_parser_level_syntax_error() {
    // HCL's grammar requires unique attribute keys, so hcl-edit rejects a
    // redefined attribute during parsing and from_hcl never sees it. Only
    // blocks, which may legally repeat, get the collected duplicate-field
    // treatment.
    let input = r#"hostname = "127.0.0.1"
port = 8080
port = 9090
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30
"#;
    let (_, report, config) = load(input);
    assert!(config.is_none());

    let issue = &report.issues()[0];
    assert!(issue.message.starts_with("syntax error:"), "got: {issue:?}");
    assert!(
        issue.message.contains("unique attribute key"),
        "got: {issue:?}"
    );
}

#[test]
fn duplicate_block_points_back_at_first_occurrence() {
    let input = r#"hostname = "127.0.0.1"
port = 8080
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30

tls {
  cert = "a.pem"
  key = "a.key"
}

tls {
  cert = "b.pem"
  key = "b.key"
}
"#;
    let (_, report, config) = load(input);
    assert!(config.is_none());

    let issue = report
        .issues()
        .iter()
        .find(|issue| issue.message == "duplicate field: tls")
        .expect("duplicate block must be reported");
    assert_eq!(issue.related.len(), 1);
    assert_eq!(issue.related[0].1, "first declared here");

    let second = issue.span.unwrap();
    let text = &input[second.start as usize..second.end as usize];
    assert!(
        text.contains("b.pem"),
        "error points at the second block: {text:?}"
    );
}

#[test]
fn lowering_failure_is_caught_not_panicking() {
    let input = r#"hostname = "not a hostname"
port = 8080
daemon = false
max_connections = 100
shutdown_timeout_seconds = 30
"#;
    let (_, report, config) = load(input);
    assert!(config.is_none());
    assert!(
        report
            .issues()
            .iter()
            .any(|issue| issue.message.starts_with("invalid address:")),
        "got: {:?}",
        report.issues()
    );
}

#[test]
fn syntax_error_reports_location_and_stops() {
    let (_, report, config) = load("port = \n");
    assert!(config.is_none());
    assert!(report.issues()[0].message.starts_with("syntax error:"));
}
