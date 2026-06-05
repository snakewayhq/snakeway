use crate::bench_api::RequestCtx;
use crate::execution::device::wasm::bindings::snakeway::device::types::{Header, Request};

pub(crate) fn build_request_snapshot(ctx: &RequestCtx) -> Request {
    Request {
        method: ctx.method_str().to_string(),
        scheme: ctx.scheme().to_string(),
        authority: ctx.effective_host().to_string(),
        original_path: ctx.original_uri_path().to_string(),
        route_path: ctx.canonical_path().to_string(),
        query: ctx.query_string().to_string(),
        client_ip: ctx.peer_ip.to_string(),
        headers: ctx
            .headers()
            .iter()
            .map(|(k, v)| Header {
                name: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            })
            .collect(),
    }
}
