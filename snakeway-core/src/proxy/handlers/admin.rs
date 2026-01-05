use crate::connection_management::ConnectionManager;
use crate::server::ReloadHandle;
use crate::traffic_management::TrafficManager;
use http::{StatusCode, header};
use pingora::prelude::Session;
use pingora::{Custom, Error};
use pingora_http::ResponseHeader;
use std::str::FromStr;
use std::sync::Arc;

#[derive(Debug, PartialEq)]
enum AdminEndpoint {
    Health,
    Upstreams,
    Stats,
    Reload,
}

impl FromStr for AdminEndpoint {
    type Err = &'static str;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "/admin/health" => Ok(AdminEndpoint::Health),
            "/admin/upstreams" => Ok(AdminEndpoint::Upstreams),
            "/admin/stats" => Ok(AdminEndpoint::Stats),
            "/admin/reload" => Ok(AdminEndpoint::Reload),
            _ => Err("invalid admin endpoint"),
        }
    }
}

pub struct AdminHandler {
    traffic_manager: Arc<TrafficManager>,
    connection_manager: Arc<ConnectionManager>,
    reload: Arc<ReloadHandle>,
}

impl AdminHandler {
    pub fn new(
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<ConnectionManager>,
        reload: Arc<ReloadHandle>,
    ) -> Self {
        Self {
            traffic_manager,
            connection_manager,
            reload,
        }
    }

    pub(crate) async fn handle(&self, session: &mut Session, path: &str) -> pingora::Result<bool> {
        let admin_endpoint = path
            .parse::<AdminEndpoint>()
            .map_err(|_| Error::new(Custom("invalid admin endpoint")))?;

        match admin_endpoint {
            AdminEndpoint::Health | AdminEndpoint::Upstreams => {
                let include_details = matches!(admin_endpoint, AdminEndpoint::Upstreams);
                let snapshot = self.traffic_manager.snapshot();
                let mut services = std::collections::HashMap::new();

                for (svc_id, svc_snapshot) in &snapshot.services {
                    let mut upstreams = std::collections::HashMap::new();
                    for u in &svc_snapshot.upstreams {
                        let view = self.traffic_manager.get_upstream_view(
                            svc_id,
                            &u.endpoint.id,
                            include_details,
                        );
                        let addr = format!("{}:{}", u.endpoint.host, u.endpoint.port);
                        upstreams.insert(addr, view);
                    }
                    services.insert(svc_id.clone(), upstreams);
                }

                let body = serde_json::to_vec(&serde_json::json!({ "services": services }))
                    .map_err(|_| Error::new(Custom("json serialization failed")))?;

                self.send_json_response(session, StatusCode::OK, body)
                    .await?;
                Ok(true)
            }

            AdminEndpoint::Stats => {
                let traffic = self.traffic_manager.snapshot();
                let mut traffic_stats = std::collections::HashMap::new();

                for (svc_id, svc_snapshot) in &traffic.services {
                    let mut svc_stats = serde_json::json!({
                        "total_requests": 0,
                        "total_successes": 0,
                        "total_failures": 0,
                        "active_requests": 0,
                    });

                    for u in &svc_snapshot.upstreams {
                        let active = self.traffic_manager.active_requests(svc_id, &u.endpoint.id);
                        let total = self.traffic_manager.total_requests(svc_id, &u.endpoint.id);
                        let successes =
                            self.traffic_manager.total_successes(svc_id, &u.endpoint.id);
                        let failures = self.traffic_manager.total_failures(svc_id, &u.endpoint.id);

                        let s = svc_stats.as_object_mut().unwrap();
                        s["active_requests"] =
                            (s["active_requests"].as_u64().unwrap() + active as u64).into();
                        s["total_requests"] =
                            (s["total_requests"].as_u64().unwrap() + total as u64).into();
                        s["total_successes"] =
                            (s["total_successes"].as_u64().unwrap() + successes as u64).into();
                        s["total_failures"] =
                            (s["total_failures"].as_u64().unwrap() + failures as u64).into();
                    }
                    traffic_stats.insert(svc_id.clone(), svc_stats);
                }

                let connections = self.connection_manager.snapshot();

                let mut ws_connections = std::collections::HashMap::new();
                for c in connections {
                    ws_connections.insert(
                        c.route_id,
                        serde_json::json!({
                            "active": c.active,
                            "max": c.max
                        }),
                    );
                }

                let body = serde_json::to_vec(&serde_json::json!({
                    "traffic": traffic_stats,
                    "connections": {
                        "websocket": ws_connections
                    }
                }))
                .map_err(|_| Error::new(Custom("json serialization failed")))?;

                self.send_json_response(session, StatusCode::OK, body)
                    .await?;
                Ok(true)
            }

            AdminEndpoint::Reload => {
                let method = session.req_header().method.clone();

                // Return early when not a POST request.
                if method != http::Method::POST {
                    let mut resp = ResponseHeader::build(StatusCode::METHOD_NOT_ALLOWED, None)?;
                    resp.insert_header(header::ALLOW, "POST")?;
                    resp.insert_header(header::CONTENT_LENGTH, "0")?;
                    session.write_response_header(Box::new(resp), true).await?;
                    return Ok(true);
                }

                let epoch = self.reload.notify_reload();

                let body = serde_json::to_vec(&serde_json::json!({
                    "message": "reload requested",
                    "epoch": epoch
                }))
                .map_err(|_| Error::new(Custom("json serialization failed")))?;

                self.send_json_response(session, StatusCode::OK, body)
                    .await?;
                Ok(true)
            }
        }
    }

    async fn send_json_response(
        &self,
        session: &mut Session,
        status: StatusCode,
        body: Vec<u8>,
    ) -> pingora::Result<()> {
        let mut resp = ResponseHeader::build(status, None)?;
        resp.insert_header(header::CONTENT_TYPE, "application/json")?;
        resp.insert_header(header::CONTENT_LENGTH, body.len().to_string())?;

        session.write_response_header(Box::new(resp), false).await?;
        session.write_response_body(Some(body.into()), true).await?;

        Ok(())
    }
}
