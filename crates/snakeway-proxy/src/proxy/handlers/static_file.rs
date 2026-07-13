use pingora::prelude::Session;
use pingora::{Custom, Error};
use snakeway_engine::ctx::RequestCtx;
use snakeway_engine::device::core::DeviceRegistry;
use snakeway_engine::route::RouteEntry;

pub(crate) struct StaticFileHandler;

impl StaticFileHandler {
    pub(crate) async fn handle(
        &self,
        session: &mut Session,
        ctx: &RequestCtx,
        route: &RouteEntry,
        devices: &DeviceRegistry,
    ) -> pingora::Result<bool> {
        use pingora::http::ResponseHeader;
        use snakeway_engine::ctx::{RequestId, ResponseCtx};
        use snakeway_engine::device::core::DevicePipeline;
        use snakeway_engine::device::core::DeviceResult;
        use tokio::io::AsyncReadExt;

        // Extract conditional headers for cache validation and content negotiation.
        let conditional = crate::static_files::ConditionalHeaders {
            if_none_match: ctx
                .headers()
                .get(http::header::IF_NONE_MATCH)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            if_modified_since: ctx
                .headers()
                .get(http::header::IF_MODIFIED_SINCE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            accept_encoding: ctx
                .headers()
                .get(http::header::ACCEPT_ENCODING)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
            range: ctx
                .headers()
                .get(http::header::RANGE)
                .and_then(|v| v.to_str().ok())
                .map(|s| s.to_string()),
        };

        let static_resp = crate::static_files::handle_static_request(
            &route.kind,
            ctx.canonical_path(),
            &conditional,
        )
        .await;

        // Build response header
        let mut resp = ResponseHeader::build(static_resp.status, None)?;

        // Copy headers
        for (name, value) in static_resp.headers.iter() {
            resp.insert_header(name, value)?;
        }

        // Write headers (not end-of-stream yet)
        session.write_response_header(Box::new(resp), false).await?;

        if ctx.method() == http::Method::HEAD {
            // Short-circuit the body write step for HEAD requests.
            session.write_response_body(None, true).await?;
        } else {
            // Write body and end the stream.
            match static_resp.body {
                crate::static_files::StaticBody::Empty => {
                    session.write_response_body(None, true).await?;
                }

                crate::static_files::StaticBody::Bytes(bytes) => {
                    session.write_response_body(Some(bytes), true).await?;
                }

                crate::static_files::StaticBody::File(mut file) => {
                    use bytes::{Bytes, BytesMut};
                    use tokio::io::AsyncReadExt;

                    const CHUNK_SIZE: usize = 32 * 1024;

                    // Allocate once per request.
                    let mut buf = BytesMut::with_capacity(CHUNK_SIZE);

                    loop {
                        // Ensure we have space to read into.
                        buf.resize(CHUNK_SIZE, 0);

                        let n = file
                            .read(&mut buf[..])
                            .await
                            .map_err(|_| Error::new(Custom("static file read error")))?;

                        if n == 0 {
                            break;
                        }

                        // Shrink to actual read size.
                        buf.truncate(n);

                        // Split off the filled bytes and freeze them.
                        let chunk: Bytes = buf.split().freeze();

                        session.write_response_body(Some(chunk), false).await?;
                    }

                    // End-of-stream.
                    session.write_response_body(None, true).await?;
                }

                crate::static_files::StaticBody::RangedFile {
                    mut file,
                    mut remaining,
                } => {
                    const CHUNK_SIZE: usize = 32 * 1024;
                    let mut buf = bytes::BytesMut::with_capacity(CHUNK_SIZE);

                    while remaining > 0 {
                        let to_read = std::cmp::min(CHUNK_SIZE as u64, remaining) as usize;

                        buf.resize(to_read, 0);

                        let n = file
                            .read(&mut buf[..])
                            .await
                            .map_err(|_| Error::new(Custom("static file read error")))?;

                        if n == 0 {
                            break;
                        }

                        remaining -= n as u64;
                        buf.truncate(n);

                        session
                            .write_response_body(Some(buf.split().freeze()), false)
                            .await?;
                    }

                    session.write_response_body(None, true).await?;
                }
            }
        }

        // Run on_response devices
        let request_id = ctx
            .extensions
            .get::<RequestId>()
            .map(|id| id.as_str().to_owned());
        let mut resp_ctx = ResponseCtx::new(
            request_id,
            static_resp.status,
            static_resp.headers,
            Vec::new(),
        );

        match DevicePipeline::run_on_response(devices.all(), &mut resp_ctx) {
            DeviceResult::Continue => {}
            DeviceResult::Respond(_) => {}
            DeviceResult::Error(err) => {
                tracing::warn!("device error on_response (static): {err}");
            }
        }

        Ok(true)
    }
}
