use anyhow::{anyhow, Error, Result};
use pingora::prelude::*;
use pingora::server::Server;

use crate::config::{RouteConfig, SnakewayConfig};
use crate::device::core::registry::DeviceRegistry;
use crate::proxy::SnakewayGateway;
use crate::route::{RouteKind, Router};

/// Run the Pingora server with the given configuration.
pub fn run(config: SnakewayConfig) -> Result<()> {
    let server = build_pingora_server(config)?;
    server.run_forever();
}

/// Build the Pingora server.
pub fn build_pingora_server(config: SnakewayConfig) -> Result<Server, Error> {
    let mut server = Server::new(None)?;
    server.bootstrap();

    // Build routing table
    let router = build_router(&config.routes)?;

    // Load devices
    let mut registry = DeviceRegistry::new();
    registry.load_from_config(&config)?;
    tracing::debug!("Loaded device count = {}", registry.all().len());

    // Extract upstream route (exactly one required)
    let upstream_route = config
        .routes
        .iter()
        .find(|r| r.upstream.is_some())
        .ok_or_else(|| anyhow!("Snakeway: exactly one upstream route is required"))?;

    let (host, port) = parse_upstream(
        upstream_route
            .upstream
            .as_ref()
            .expect("validated upstream"),
    )?;

    // Build gateway
    let gateway = SnakewayGateway {
        upstream_host: host,
        upstream_port: port,
        use_tls: false,     // HTTP only (todo)
        sni: String::new(), // no SNI (todo)

        router,
        devices: registry,
    };

    // Build HTTP proxy service from Pingora.
    let mut svc = http_proxy_service(&server.configuration, gateway);
    svc.add_tcp(&config.server.listen);

    // Register service.
    server.add_service(svc);

    Ok(server)
}

/// Build router from config routes.
fn build_router(routes: &[RouteConfig]) -> Result<Router> {
    let mut router = Router::new();

    for route in routes {
        let kind = if let Some(upstream) = &route.upstream {
            RouteKind::Proxy {
                upstream: upstream.clone(),
            }
        } else if let Some(dir) = &route.file_dir {
            RouteKind::Static {
                path: route.path.clone(),
                file_dir: dir.into(),
                index: route.index,
            }
        } else {
            unreachable!("route validation should prevent this");
        };

        router.add_route(&route.path, kind)?;
    }

    Ok(router)
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
