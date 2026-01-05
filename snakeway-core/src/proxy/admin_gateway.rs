use crate::ctx::RequestCtx;
use crate::proxy::handlers::AdminHandler;
use crate::server::ReloadHandle;
use crate::traffic_management::TrafficManager;
use crate::ws_connection_management::WsConnectionManager;
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
    ) -> Self {
        Self {
            admin_handler: AdminHandler::new(traffic_manager, connection_manager, reload),
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
