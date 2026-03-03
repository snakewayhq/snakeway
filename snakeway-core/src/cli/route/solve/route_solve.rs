use crate::cli::route::args::{RouteSolveArgs, RouteSolveOutputFormat};
use crate::cli::route::solve::solver::solve;
use crate::cli::route::solve::types::{RouteSolveDecision, RouteSolveOptions, SyntheticRequest};
use crate::conf::load_config;
use crate::runtime::build_runtime_state;
use http::{HeaderMap, HeaderName, HeaderValue, Method};
use std::net::IpAddr;
use std::process;
use url::Url;

/// Exit codes per specification.
const EXIT_OK: i32 = 0;
const EXIT_BAD_INPUT: i32 = 1;
const EXIT_CONFIG_FAILURE: i32 = 2;
const EXIT_NO_MATCH: i32 = 3;
const EXIT_REJECTED: i32 = 4;

pub fn run(args: RouteSolveArgs) {
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

    // 6. Load config
    let validated = match load_config(&args.config) {
        Ok(v) => v,
        Err(e) => {
            eprintln!(
                "error: failed to load config from '{}': {}",
                args.config.display(),
                e
            );
            process::exit(EXIT_CONFIG_FAILURE);
        }
    };

    if !validated.is_valid() {
        validated.validation_report.render_pretty();
        eprintln!("error: configuration has validation errors");
        process::exit(EXIT_CONFIG_FAILURE);
    }

    // 7. Build runtime state (no cert manager needed for dry-run)
    let state = match build_runtime_state(&validated.config, &None) {
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
    let decision = solve(&state, &req, &opts);

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
    let json = serde_json::to_string_pretty(decision).expect("failed to serialize decision");
    println!("{}", json);
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
