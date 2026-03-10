use crate::execution::traffic::TrafficManager;
use crate::runtime::UpstreamRuntime;
use pingora::{Custom, Error};
use std::sync::Arc;

pub fn admin_health(
    traffic_manager: Arc<TrafficManager>,
    include_details: bool,
) -> Result<Vec<u8>, Box<dyn std::error::Error>> {
    let mut services = std::collections::HashMap::new();
    let snapshot = traffic_manager.snapshot();

    for (svc_id, svc_snapshot) in &snapshot.services {
        let mut tcp_upstreams = std::collections::HashMap::new();
        for u in &svc_snapshot.upstreams {
            match &u.endpoint {
                UpstreamRuntime::Tcp(tcp) => {
                    let view = traffic_manager.get_upstream_view(
                        svc_id,
                        &u.endpoint.id(),
                        include_details,
                    );
                    let addr = format!("{}:{}", tcp.host, tcp.port);
                    tcp_upstreams.insert(addr, view);
                }
                UpstreamRuntime::Unix(_) => {}
            };
        }
        services.insert(svc_id.clone(), tcp_upstreams);
    }

    let body = serde_json::to_vec(&serde_json::json!({ "services": services }))
        .map_err(|_| Error::new(Custom("json serialization failed")))?;
    Ok(body)
}
