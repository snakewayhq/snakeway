use crate::execution::ctx::{RequestCtx, RequestId, ResponseCtx};
use crate::execution::device::core::DeviceResult;
use crate::execution::device::wasm::bindings::exports::snakeway::device::policy;
use crate::execution::device::wasm::bindings::snakeway::device::types::{
    Action, BodyAction, Header, HeaderOp, Request, RequestPatch, Response, ResponsePatch,
};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

pub(crate) fn apply_body_result(
    request_id: Option<String>,
    maybe_chunk: &mut Option<Bytes>,
    result: policy::BodyResult,
) -> anyhow::Result<DeviceResult> {
    match result.action {
        BodyAction::Passthrough => Ok(DeviceResult::Continue),
        BodyAction::Replace(data) => {
            *maybe_chunk = Some(Bytes::from(data));
            Ok(DeviceResult::Continue)
        }
        BodyAction::Drop => {
            *maybe_chunk = None;
            Ok(DeviceResult::Continue)
        }
        BodyAction::Block => Ok(DeviceResult::Respond(block_403(request_id))),
    }
}

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

pub(crate) fn build_response_snapshot(ctx: &ResponseCtx) -> Response {
    Response {
        status: ctx.status.as_u16(),
        headers: ctx
            .headers
            .iter()
            .map(|(k, v)| Header {
                name: k.to_string(),
                value: v.to_str().unwrap_or("").to_string(),
            })
            .collect(),
    }
}

fn handle_action(action: Action, request_id: Option<String>) -> anyhow::Result<DeviceResult> {
    match action {
        Action::Continue => Ok(DeviceResult::Continue),
        Action::Block => Ok(DeviceResult::Respond(block_403(request_id))),
        Action::Respond(synthetic) => {
            let status =
                StatusCode::from_u16(synthetic.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut headers = HeaderMap::new();
            for h in synthetic.headers {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.insert(name, value);
                }
            }
            Ok(DeviceResult::Respond(ResponseCtx::new(
                request_id,
                status,
                headers,
                synthetic.body,
            )))
        }
    }
}

fn apply_request_header_ops(ctx: &mut RequestCtx, ops: Vec<HeaderOp>) {
    for op in ops {
        match op {
            HeaderOp::Set(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    ctx.insert_header(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    ctx.append_header(name, value);
                }
            }
            HeaderOp::Remove(name) => {
                ctx.remove_header(&name);
            }
        }
    }
}

fn apply_header_ops(headers: &mut HeaderMap, ops: Vec<HeaderOp>) {
    for op in ops {
        match op {
            HeaderOp::Set(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.insert(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let (Ok(name), Ok(value)) = (
                    h.name.parse::<HeaderName>(),
                    HeaderValue::from_str(&h.value),
                ) {
                    headers.append(name, value);
                }
            }
            HeaderOp::Remove(name) => {
                if let Ok(name) = name.parse::<HeaderName>() {
                    headers.remove(name);
                }
            }
        }
    }
}

pub(crate) fn apply_request_result(
    ctx: &mut RequestCtx,
    result: policy::RequestResult,
) -> anyhow::Result<DeviceResult> {
    let request_id = ctx.extensions.get::<RequestId>().map(|id| id.0.clone());

    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, request_id);
    }

    if let Some(RequestPatch {
        set_route_path,
        set_upstream_path,
        ops,
    }) = result.patch
    {
        if let Some(path) = set_route_path {
            ctx.set_canonical_path(path);
        }
        if let Some(path) = set_upstream_path {
            ctx.upstream_path = Some(path);
        }
        apply_request_header_ops(ctx, ops);
    }

    Ok(DeviceResult::Continue)
}

pub(crate) fn apply_response_result(
    ctx: &mut ResponseCtx,
    result: policy::ResponseResult,
) -> anyhow::Result<DeviceResult> {
    let request_id = ctx.request_id.clone();

    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, request_id);
    }

    if let Some(ResponsePatch { set_status, ops }) = result.patch {
        if let Some(status_code) = set_status.and_then(|s| StatusCode::from_u16(s).ok()) {
            ctx.status = status_code;
        }
        apply_header_ops(&mut ctx.headers, ops);
    }

    Ok(DeviceResult::Continue)
}

pub(crate) fn block_403(request_id: Option<String>) -> ResponseCtx {
    ResponseCtx::new(
        request_id,
        StatusCode::FORBIDDEN,
        HeaderMap::new(),
        b"Blocked by device".to_vec(),
    )
}

pub(crate) fn block_503(request_id: Option<String>) -> ResponseCtx {
    ResponseCtx::new(
        request_id,
        StatusCode::SERVICE_UNAVAILABLE,
        HeaderMap::new(),
        b"Service unavailable".to_vec(),
    )
}
