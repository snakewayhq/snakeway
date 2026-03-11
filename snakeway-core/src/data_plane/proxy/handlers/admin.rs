use crate::control_plane::acme::CertManager;
use crate::control_plane::reload::ReloadHandle;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::traffic::TrafficManager;
use crate::runtime::UpstreamRuntime;

use http::{Method, StatusCode, header};
use pingora::http::ResponseHeader;
use pingora::prelude::Session;
use pingora::{Custom, Error};

use std::collections::HashMap;
use std::sync::Arc;

/// Group dependencies for the admin handler.
pub struct AdminContext {
    pub traffic: Arc<TrafficManager>,
    pub ws: Arc<WsConnectionManager>,
    pub reload: Arc<ReloadHandle>,
    pub certs: Option<Arc<CertManager>>,
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
            let mut tcp_upstreams = HashMap::new();

            for u in &svc_snapshot.upstreams {
                let view = self
                    .ctx
                    .traffic
                    .get_upstream_view(svc_id, &u.endpoint.id(), true);

                let key = match &u.endpoint {
                    UpstreamRuntime::Tcp(tcp) => {
                        format!("{}:{}", tcp.host, tcp.port)
                    }

                    UpstreamRuntime::Unix(unix) => {
                        format!("unix:{}", unix.path)
                    }
                };

                tcp_upstreams.insert(key, view);
            }

            services.insert(svc_id.clone(), tcp_upstreams);
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
            let mut total_requests = 0;
            let mut total_successes = 0;
            let mut total_failures = 0;
            let mut active_requests = 0;

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
            .map_err(|e| Error::new(Custom(&format!("json serialization failed: {e}"))))?;

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
