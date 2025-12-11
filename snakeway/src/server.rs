use anyhow::{Result, anyhow};
use pingora::prelude::*;
use pingora::server::Server;

use crate::config::SnakewayConfig;
use crate::proxy::SnakewayGateway;

/// Run the Pingora server with the given configuration.
pub fn run(config: SnakewayConfig) -> Result<()> {
    // Create a basic Pingora server.
    let mut server = Server::new(None)?;
    server.bootstrap();

    // Create a gateway from the first route.
    let route = config
        .routes
        .first()
        .ok_or_else(|| anyhow!("Snakeway: at least one route is required"))?;

    let (host, port) = parse_upstream(&route.upstream)?;

    let gateway = SnakewayGateway {
        upstream_host: host,
        upstream_port: port,
        use_tls: false,     // HTTP only
        sni: String::new(), // no SNI (yet)
    };

    // Build HTTP proxy service from Pingora.
    let mut svc = http_proxy_service(&server.configuration, gateway);
    svc.add_tcp(&config.server.listen);

    // Register service and block forever.
    server.add_service(svc);
    server.run_forever();
}

/// Parse an upstream address of the form "host:port".
fn parse_upstream(s: &str) -> Result<(String, u16)> {
    let mut parts = s.split(':');
    let host = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", s))?;
    let port = parts
        .next()
        .ok_or_else(|| anyhow!("invalid upstream address: {}", s))?
        .parse::<u16>()?;

    Ok((host.to_string(), port))
}
