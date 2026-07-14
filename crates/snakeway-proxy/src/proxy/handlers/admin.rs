use snakeway_acme::CertManager;
use snakeway_engine::WsConnectionManager;
use snakeway_engine::runtime::UpstreamRuntime;
use snakeway_engine::traffic::TrafficManager;

use http::{Method, StatusCode, header};
use pingora::http::ResponseHeader;
use pingora::prelude::Session;
use pingora::{Custom, Error};
use snakeway_conf::types::AdminAuthConfig;

use crate::reload::ReloadHandle;
use std::collections::HashMap;
use std::sync::Arc;

/// Realm advertised on every 401 response from the admin API.
const ADMIN_AUTH_REALM: &str = "snakeway-admin";

/// Group dependencies for the admin handler.
pub struct AdminContext {
    pub traffic: Arc<TrafficManager>,
    pub ws: Arc<WsConnectionManager>,
    pub reload: Arc<ReloadHandle>,
    pub certs: Option<Arc<CertManager>>,
    pub auth: Arc<AdminAuthConfig>,
}

/// Collects admin handler routes
#[derive(Debug, Clone, Copy)]
enum AdminEndpoint {
    Health,
    Upstreams,
    Stats,
    Reload,
    Certs,
}

impl AdminEndpoint {
    fn from_path(path: &str) -> Option<Self> {
        match path.trim_end_matches('/') {
            "/admin/health" => Some(Self::Health),
            "/admin/upstreams" => Some(Self::Upstreams),
            "/admin/stats" => Some(Self::Stats),
            "/admin/reload" => Some(Self::Reload),
            "/admin/certs" => Some(Self::Certs),
            _ => None,
        }
    }
}

// AdminHandler
pub struct AdminHandler {
    ctx: Arc<AdminContext>,
}

impl AdminHandler {
    pub fn new(ctx: Arc<AdminContext>) -> Self {
        Self { ctx }
    }

    pub async fn handle(&self, session: &mut Session, path: &str) -> pingora::Result<bool> {
        // Authenticate before any endpoint dispatch. This runs before path
        // resolution so that unauthenticated callers cannot probe which
        // endpoints exist.
        if !self.authenticate(session).await? {
            return Ok(true);
        }

        let endpoint = AdminEndpoint::from_path(path)
            .ok_or_else(|| Error::new(Custom("invalid admin endpoint")))?;

        match endpoint {
            AdminEndpoint::Health => self.health(session).await,
            AdminEndpoint::Upstreams => self.upstreams(session).await,
            AdminEndpoint::Stats => self.stats(session).await,
            AdminEndpoint::Reload => self.reload(session).await,
            AdminEndpoint::Certs => self.certs(session).await,
        }
    }
}

/// Authentication
impl AdminHandler {
    /// Returns `Ok(true)` if the caller is authenticated and the pipeline
    /// should continue. Returns `Ok(false)` after writing a `401 Unauthorized`
    /// response; callers must short-circuit and not dispatch any endpoint.
    async fn authenticate(&self, session: &mut Session) -> pingora::Result<bool> {
        let Some(bearer) = &self.ctx.auth.bearer else {
            // Validation guarantees a scheme is configured on admin listeners.
            // If we got here without one, fail closed rather than silently
            // allowing access.
            tracing::warn!("admin listener has no auth scheme configured; rejecting request");
            self.respond_unauthorized(session).await?;
            return Ok(false);
        };

        let headers = &session.req_header().headers;
        let Some(value) = headers.get(header::AUTHORIZATION) else {
            tracing::warn!(
                path = session.req_header().uri.path(),
                reason = "missing_header",
                "admin auth failed"
            );
            self.respond_unauthorized(session).await?;
            return Ok(false);
        };

        let Ok(raw) = value.to_str() else {
            tracing::warn!(
                path = session.req_header().uri.path(),
                reason = "non_ascii_header",
                "admin auth failed"
            );
            self.respond_unauthorized(session).await?;
            return Ok(false);
        };

        // Split on the first whitespace run: "<scheme> <token>".
        let mut parts = raw.splitn(2, char::is_whitespace);
        let scheme = parts.next().unwrap_or("");
        let token = parts.next().unwrap_or("").trim();

        if !scheme.eq_ignore_ascii_case("Bearer") {
            tracing::warn!(
                path = session.req_header().uri.path(),
                reason = "wrong_scheme",
                "admin auth failed"
            );
            self.respond_unauthorized(session).await?;
            return Ok(false);
        }

        if token.is_empty() {
            tracing::warn!(
                path = session.req_header().uri.path(),
                reason = "empty_token",
                "admin auth failed"
            );
            self.respond_unauthorized(session).await?;
            return Ok(false);
        }

        if !bearer.verify(token.as_bytes()) {
            tracing::warn!(
                path = session.req_header().uri.path(),
                reason = "invalid_token",
                "admin auth failed"
            );
            self.respond_unauthorized(session).await?;
            return Ok(false);
        }

        Ok(true)
    }

    async fn respond_unauthorized(&self, session: &mut Session) -> pingora::Result<()> {
        let mut resp = ResponseHeader::build(StatusCode::UNAUTHORIZED, None)?;
        resp.insert_header(
            header::WWW_AUTHENTICATE,
            format!("Bearer realm=\"{ADMIN_AUTH_REALM}\""),
        )?;
        resp.insert_header(header::CONTENT_LENGTH, "0")?;
        session.write_response_header(Box::new(resp), true).await?;
        Ok(())
    }
}

/// Endpoints
impl AdminHandler {
    async fn health(&self, session: &mut Session) -> pingora::Result<bool> {
        self.json(
            session,
            StatusCode::OK,
            serde_json::json!({ "status": "ok" }),
        )
        .await?;

        Ok(true)
    }

    async fn upstreams(&self, session: &mut Session) -> pingora::Result<bool> {
        let snapshot = self.ctx.traffic.snapshot();

        let mut services = HashMap::new();

        for (svc_id, svc_snapshot) in &snapshot.services {
            let mut upstreams = HashMap::new();

            for u in &svc_snapshot.upstreams {
                let endpoint_label = match &u.endpoint {
                    UpstreamRuntime::Tcp(tcp) => {
                        format!("{}:{}", tcp.host, tcp.port)
                    }

                    UpstreamRuntime::Unix(unix) => {
                        format!("unix:{}", unix.path)
                    }
                };
                let view = self
                    .ctx
                    .traffic
                    .get_upstream_view(svc_id, u, &endpoint_label, true);

                upstreams.insert(endpoint_label, view);
            }

            services.insert(svc_id.clone(), upstreams);
        }

        self.json(
            session,
            StatusCode::OK,
            serde_json::json!({ "services": services }),
        )
        .await?;

        Ok(true)
    }

    async fn stats(&self, session: &mut Session) -> pingora::Result<bool> {
        let traffic = self.ctx.traffic.snapshot();

        let mut traffic_stats = HashMap::new();

        for (svc_id, svc_snapshot) in &traffic.services {
            let mut total_requests: u64 = 0;
            let mut total_successes: u64 = 0;
            let mut total_failures: u64 = 0;
            let mut active_requests: u64 = 0;

            for u in &svc_snapshot.upstreams {
                let id = u.endpoint.id();

                active_requests += self.ctx.traffic.active_requests(svc_id, &id);
                total_requests += self.ctx.traffic.total_requests(svc_id, &id);
                total_successes += self.ctx.traffic.total_successes(svc_id, &id);
                total_failures += self.ctx.traffic.total_failures(svc_id, &id);
            }

            traffic_stats.insert(
                svc_id.clone(),
                serde_json::json!({
                    "active_requests": active_requests,
                    "total_requests": total_requests,
                    "total_successes": total_successes,
                    "total_failures": total_failures
                }),
            );
        }

        let ws_connections = self.ctx.ws.snapshot();

        self.json(
            session,
            StatusCode::OK,
            serde_json::json!({
                "traffic": traffic_stats,
                "websocket": ws_connections
            }),
        )
        .await?;

        Ok(true)
    }

    async fn reload(&self, session: &mut Session) -> pingora::Result<bool> {
        if session.req_header().method != Method::POST {
            return self.method_not_allowed(session, "POST").await;
        }

        let epoch = self.ctx.reload.notify_reload();

        self.json(
            session,
            StatusCode::OK,
            serde_json::json!({
                "message": "reload requested",
                "epoch": epoch
            }),
        )
        .await?;

        Ok(true)
    }

    async fn certs(&self, session: &mut Session) -> pingora::Result<bool> {
        let certs = match &self.ctx.certs {
            Some(mgr) => mgr.snapshot(),
            None => Vec::new(),
        };

        self.json(
            session,
            StatusCode::OK,
            serde_json::json!({ "certs": certs }),
        )
        .await?;

        Ok(true)
    }
}

/// Response helpers
impl AdminHandler {
    async fn json(
        &self,
        session: &mut Session,
        status: StatusCode,
        body: serde_json::Value,
    ) -> pingora::Result<()> {
        let body = serde_json::to_vec(&body)
            .map_err(|_| Error::new(Custom("failed to serialize json response")))?;

        let mut resp = ResponseHeader::build(status, None)?;

        resp.insert_header(header::CONTENT_TYPE, "application/json")?;
        resp.insert_header(header::CONTENT_LENGTH, body.len().to_string())?;

        session.write_response_header(Box::new(resp), false).await?;
        session.write_response_body(Some(body.into()), true).await?;

        Ok(())
    }

    async fn method_not_allowed(
        &self,
        session: &mut Session,
        allowed: &str,
    ) -> pingora::Result<bool> {
        let mut resp = ResponseHeader::build(StatusCode::METHOD_NOT_ALLOWED, None)?;

        resp.insert_header(header::ALLOW, allowed)?;
        resp.insert_header(header::CONTENT_LENGTH, "0")?;

        session.write_response_header(Box::new(resp), true).await?;

        Ok(true)
    }
}
