use crate::control_plane::acme::CertManager;
use crate::control_plane::reload::ReloadHandle;
use crate::data_plane::proxy::handlers::AdminHandler;
use crate::data_plane::ws_connection_management::WsConnectionManager;
use crate::execution::ctx::RequestCtx;
use crate::execution::traffic::TrafficManager;
use async_trait::async_trait;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::{Custom, Error};
use std::sync::Arc;

pub struct AdminGateway {
    admin_handler: AdminHandler,
}

impl AdminGateway {
    pub fn new(
        traffic_manager: Arc<TrafficManager>,
        connection_manager: Arc<WsConnectionManager>,
        reload: Arc<ReloadHandle>,
        cert_manager: Option<Arc<CertManager>>,
    ) -> Self {
        Self {
            admin_handler: AdminHandler::new(
                traffic_manager,
                connection_manager,
                reload,
                cert_manager,
            ),
        }
    }
}

#[async_trait]
impl ProxyHttp for AdminGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Minimal ctx - admin requests never enter the proxy lifecycle.
        RequestCtx::empty()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // This is unreachable by design.
        Err(Error::new(Custom(
            "AdminGateway attempted to proxy upstream (bug)",
        )))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        // AdminGateway is terminal: it always handles the request.
        let path = session.req_header().uri.path().to_owned();
        self.admin_handler.handle(session, &path).await
    }
}
