use async_trait::async_trait;
use bytes::Bytes;
use pingora::http::ResponseHeader;
use pingora::prelude::{HttpPeer, ProxyHttp, Session};
use pingora::{Custom, Error};
use snakeway_acme::CertManager;
use snakeway_engine::ctx::RequestCtx;
use std::sync::Arc;

pub(crate) struct RedirectGateway {
    destination: String,
    response_code: u16,
    cert_manager: Option<Arc<CertManager>>,
}

impl RedirectGateway {
    pub(crate) fn new(
        to: String,
        response_code: u16,
        cert_manager: Option<Arc<CertManager>>,
    ) -> Self {
        Self {
            destination: to,
            response_code,
            cert_manager,
        }
    }
}

#[async_trait]
impl ProxyHttp for RedirectGateway {
    type CTX = RequestCtx;

    fn new_ctx(&self) -> Self::CTX {
        // Minimal ctx - redirect requests never enter the proxy lifecycle.
        RequestCtx::empty()
    }

    async fn upstream_peer(
        &self,
        _session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<Box<HttpPeer>> {
        // This is unreachable by design.
        Err(Error::new(Custom(
            "RedirectGateway attempted to proxy upstream (bug)",
        )))
    }

    async fn request_filter(
        &self,
        session: &mut Session,
        _ctx: &mut Self::CTX,
    ) -> pingora::Result<bool> {
        //---------------------------------------------------------------------
        // Handle HTTP-01 challenge.
        //---------------------------------------------------------------------
        if let Some(cert_manager) = &self.cert_manager {
            let uri = &session.req_header().uri;
            let path = uri.path();

            const PREFIX: &str = "/.well-known/acme-challenge/";

            if let Some(token) = path.strip_prefix(PREFIX)
                && let Some(key_auth) = cert_manager.http01().get(token)
            {
                let mut resp = ResponseHeader::build(200, None)?;
                resp.insert_header("Content-Type", "text/plain")?;
                resp.insert_header("Content-Length", key_auth.len().to_string())?;
                resp.insert_header("Connection", "close")?;

                session.write_response_header(Box::new(resp), false).await?;
                let body = Bytes::from(key_auth);
                session.write_response_body(Some(body), true).await?;

                return Ok(true);
            }
        }

        //---------------------------------------------------------------------
        // RedirectGateway is terminal: it always handles the request.
        //---------------------------------------------------------------------
        let mut resp = ResponseHeader::build(self.response_code, None)?;

        // Set the redirect destination via the location header.
        let path_and_query = session
            .req_header()
            .uri
            .path_and_query()
            .map(|pq| pq.as_str())
            .unwrap_or("/");
        let location = format!("https://{}{}", self.destination, path_and_query);
        resp.insert_header("Location", &location)?;
        resp.insert_header("Connection", "close")?;
        resp.insert_header("Content-Length", "0")?;

        session.write_response_header(Box::new(resp), true).await?;

        Ok(true)
    }
}
