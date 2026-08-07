use crate::cli::route::args::{RouteSolveArgs, RouteSolveOutputFormat};
use crate::cli::route::solve::solver::walk_solve;
use crate::cli::route::solve::types::{RouteSolveDecision, RouteSolveOptions, SyntheticRequest};
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use snakeway_conf::load_config;
use snakeway_engine::runtime::build_runtime_state;
use std::net::IpAddr;
use std::process;
use url::Url;

/// Exit codes per specification.
const EXIT_OK: i32 = 0;
const EXIT_BAD_INPUT: i32 = 1;
const EXIT_CONFIG_FAILURE: i32 = 2;
const EXIT_NO_MATCH: i32 = 3;
const EXIT_REJECTED: i32 = 4;

pub(crate) fn run(args: RouteSolveArgs) {
    // 1. Parse URL
    let parsed_url = match Url::parse(&args.url) {
        Ok(u) => u,
        Err(e) => {
            eprintln!("error: invalid URL '{}': {}", args.url, e);
            process::exit(EXIT_BAD_INPUT);
        }
    };

    // 2. Parse method
    let method = match args.method.to_uppercase().parse::<Method>() {
        Ok(m) => m,
        Err(e) => {
            eprintln!("error: invalid HTTP method '{}': {}", args.method, e);
            process::exit(EXIT_BAD_INPUT);
        }
    };

    // 3. Parse headers
    let mut headers = HeaderMap::new();
    for raw in &args.headers {
        let (key, value) = match parse_header(raw) {
            Ok(pair) => pair,
            Err(e) => {
                eprintln!("error: invalid header '{}': {}", raw, e);
                process::exit(EXIT_BAD_INPUT);
            }
        };
        headers.insert(key, value);
    }

    // 4. Parse client IP
    let client_ip: Option<IpAddr> = match &args.client_ip {
        Some(ip_str) => match ip_str.parse() {
            Ok(ip) => Some(ip),
            Err(e) => {
                eprintln!("error: invalid client IP '{}': {}", ip_str, e);
                process::exit(EXIT_BAD_INPUT);
            }
        },
        None => None,
    };

    // 5. Derive/override request fields
    let scheme = args
        .scheme
        .clone()
        .unwrap_or_else(|| parsed_url.scheme().to_string());

    let host = parsed_url.host_str().unwrap_or("localhost").to_string();

    let path = args
        .path
        .clone()
        .unwrap_or_else(|| parsed_url.path().to_string());

    let query = args
        .query
        .clone()
        .or_else(|| parsed_url.query().map(|q| q.to_string()));

    let config = match load_config(&args.config) {
        Ok(validated) => validated.config,
        Err(e) => {
            eprintln!(
                "error: failed to load config from '{}': {}",
                args.config.display(),
                e
            );
            process::exit(EXIT_CONFIG_FAILURE);
        }
    };

    let state = match build_runtime_state(&config, &None) {
        Ok(s) => s,
        Err(e) => {
            eprintln!("error: failed to build runtime state: {}", e);
            process::exit(EXIT_CONFIG_FAILURE);
        }
    };

    // 8. Build synthetic request
    let verbose = args.verbose;
    let trace_enabled = args.trace || verbose;

    let req = SyntheticRequest {
        scheme,
        host,
        method,
        path,
        query,
        client_ip,
        body_size: args.body_size,
    };

    let opts = RouteSolveOptions {
        lb_key: args.lb_key,
        lb_index: args.lb_index,
        trace: trace_enabled,
        verbose,
    };

    // 9. Solve
    let decision = walk_solve(&state, &req, &opts);

    // 10. Render output
    match args.format {
        RouteSolveOutputFormat::Pretty => render_pretty(&decision, verbose),
        RouteSolveOutputFormat::Json => render_json(&decision),
    }

    // 11. Exit code
    let code = exit_code(&decision);
    process::exit(code);
}

fn parse_header(raw: &str) -> Result<(HeaderName, HeaderValue), String> {
    let colon_pos = raw
        .find(':')
        .ok_or_else(|| "missing ':' separator".to_string())?;

    let key = raw[..colon_pos].trim();
    let value = raw[colon_pos + 1..].trim();

    let name = HeaderName::from_bytes(key.as_bytes())
        .map_err(|e| format!("invalid header name: {}", e))?;
    let val = HeaderValue::from_str(value).map_err(|e| format!("invalid header value: {}", e))?;

    Ok((name, val))
}

fn render_pretty(decision: &RouteSolveDecision, verbose: bool) {
    if verbose {
        println!("Normalized Request:");
        println!("  scheme:    {}", decision.normalized.scheme);
        println!("  host:      {}", decision.normalized.host);
        println!("  method:    {}", decision.normalized.method);
        println!("  path:      {}", decision.normalized.path);
        println!(
            "  query:     {}",
            decision.normalized.query.as_deref().unwrap_or("(none)")
        );
        println!(
            "  client_ip: {}",
            decision.normalized.client_ip.as_deref().unwrap_or("(none)")
        );
        println!("  body_size: {}", decision.normalized.body_size);
        println!();
    }

    println!("Route Solve Result:");

    if let Some(ref rejection) = decision.rejection {
        println!("  status:    REJECTED");
        println!("  stage:     {}", rejection.stage);
        println!("  reason:    {}", rejection.reason);
    } else if decision.matched_route.is_some() {
        println!("  status:    RESOLVED");
        println!(
            "  route:     {}",
            decision.matched_route.as_deref().unwrap_or("(none)")
        );
        println!(
            "  kind:      {}",
            decision.route_kind.as_deref().unwrap_or("(unknown)")
        );

        if let Some(ref svc) = decision.upstream_service {
            println!("  service:   {}", svc);
        }
        if let Some(ref upstream) = decision.selected_upstream {
            println!("  upstream:  {}", upstream);
        }
        if let Some(ref dir) = decision.static_file_dir {
            println!("  file_dir:  {}", dir);
        }
    } else {
        println!("  status:    NO MATCH");
    }

    if let Some(ref trace) = decision.trace {
        println!();
        println!("Trace:");
        for step in trace {
            println!("  [{}] {} - {}", step.stage, step.outcome, step.detail);
        }
    }
}

fn render_json(decision: &RouteSolveDecision) {
    match serde_json::to_string_pretty(decision) {
        Ok(json) => println!("{json}"),
        Err(e) => {
            eprintln!("failed to serialize decision: {e}");
            process::exit(EXIT_CONFIG_FAILURE);
        }
    }
}

fn exit_code(decision: &RouteSolveDecision) -> i32 {
    if decision.rejection.is_some() {
        EXIT_REJECTED
    } else if decision.matched_route.is_none() {
        EXIT_NO_MATCH
    } else {
        EXIT_OK
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cli::route::solve::types::{RouteSolveNormalized, RouteSolveRejection};

    fn decision(
        matched_route: Option<&str>,
        rejection: Option<RouteSolveRejection>,
    ) -> RouteSolveDecision {
        RouteSolveDecision {
            matched_route: matched_route.map(str::to_string),
            route_kind: None,
            upstream_service: None,
            selected_upstream: None,
            static_file_dir: None,
            rejection,
            normalized: RouteSolveNormalized {
                scheme: "http".to_string(),
                host: "example.test".to_string(),
                method: "GET".to_string(),
                path: "/".to_string(),
                query: None,
                client_ip: None,
                body_size: 0,
            },
            trace: None,
        }
    }

    #[test]
    fn should_parse_header_name_and_value() {
        // Arrange
        let raw = "X-Test: hello";

        // Act
        let (name, value) = parse_header(raw).expect("header must parse");

        // Assert
        assert_eq!(name.as_str(), "x-test");
        assert_eq!(value.to_str().unwrap(), "hello");
    }

    #[test]
    fn should_reject_header_without_separator() {
        // Arrange
        let raw = "no-separator";

        // Act
        let result = parse_header(raw);

        // Assert
        assert!(result.expect_err("must reject").contains("missing ':'"));
    }

    #[test]
    fn should_exit_4_for_a_rejected_request() {
        // Arrange
        let decision = decision(
            Some("route"),
            Some(RouteSolveRejection {
                stage: "network_policy".to_string(),
                reason: "denied".to_string(),
            }),
        );

        // Act
        let code = exit_code(&decision);

        // Assert
        assert_eq!(code, 4);
    }

    #[test]
    fn should_exit_3_when_no_route_matches() {
        // Arrange
        let decision = decision(None, None);

        // Act
        let code = exit_code(&decision);

        // Assert
        assert_eq!(code, 3);
    }

    #[test]
    fn should_exit_0_for_a_resolved_route() {
        // Arrange
        let decision = decision(Some("route"), None);

        // Act
        let code = exit_code(&decision);

        // Assert
        assert_eq!(code, 0);
    }
}
