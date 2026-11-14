use crate::execution::ctx::{RequestCtx, ResponseCtx};
use crate::execution::device::core::DeviceResult;
use crate::execution::device::wasm::bindings::exports::snakeway::device::policy;
use crate::execution::device::wasm::bindings::snakeway::device::types::{
    Action, BodyAction, Header, HeaderOp, Request, RequestPatch, Response, ResponsePatch,
};
use bytes::Bytes;
use http::{HeaderMap, HeaderName, HeaderValue, StatusCode};

#[hotpath::measure]
pub(crate) fn apply_body_result(
    request_id: Option<&str>,
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

#[hotpath::measure]
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

#[hotpath::measure]
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

fn handle_action(action: Action, request_id: Option<&str>) -> anyhow::Result<DeviceResult> {
    match action {
        Action::Continue => Ok(DeviceResult::Continue),
        Action::Block => Ok(DeviceResult::Respond(block_403(request_id))),
        Action::Respond(synthetic) => {
            let status =
                StatusCode::from_u16(synthetic.status).unwrap_or(StatusCode::INTERNAL_SERVER_ERROR);
            let mut headers = HeaderMap::new();
            for h in &synthetic.headers {
                if let Some((name, value)) = parse_header(h) {
                    headers.insert(name, value);
                }
            }
            Ok(DeviceResult::Respond(ResponseCtx::new(
                request_id.map(str::to_owned),
                status,
                headers,
                synthetic.body,
            )))
        }
    }
}

fn parse_header(h: &Header) -> Option<(HeaderName, HeaderValue)> {
    let name = match h.name.parse::<HeaderName>() {
        Ok(n) => n,
        Err(_) => {
            tracing::warn!(header_name = %h.name, "wasm device emitted invalid header name, skipping");
            return None;
        }
    };
    let value = match HeaderValue::from_str(&h.value) {
        Ok(v) => v,
        Err(_) => {
            tracing::warn!(header_name = %h.name, "wasm device emitted invalid header value, skipping");
            return None;
        }
    };
    Some((name, value))
}

fn apply_request_header_ops(ctx: &mut RequestCtx, ops: Vec<HeaderOp>) {
    for op in ops {
        match op {
            HeaderOp::Set(h) => {
                if let Some((name, value)) = parse_header(&h) {
                    ctx.insert_header(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let Some((name, value)) = parse_header(&h) {
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
                if let Some((name, value)) = parse_header(&h) {
                    headers.insert(name, value);
                }
            }
            HeaderOp::Append(h) => {
                if let Some((name, value)) = parse_header(&h) {
                    headers.append(name, value);
                }
            }
            HeaderOp::Remove(name) => match name.parse::<HeaderName>() {
                Ok(name) => {
                    headers.remove(name);
                }
                Err(_) => {
                    tracing::warn!(header_name = %name, "wasm device emitted invalid header name for removal, skipping");
                }
            },
        }
    }
}

#[hotpath::measure]
pub(crate) fn apply_request_result(
    ctx: &mut RequestCtx,
    result: policy::RequestResult,
) -> anyhow::Result<DeviceResult> {
    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, ctx.request_id());
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

#[hotpath::measure]
pub(crate) fn apply_response_result(
    ctx: &mut ResponseCtx,
    result: policy::ResponseResult,
) -> anyhow::Result<DeviceResult> {
    if !matches!(result.action, Action::Continue) {
        return handle_action(result.action, ctx.request_id.as_deref());
    }

    if let Some(ResponsePatch { set_status, ops }) = result.patch {
        if let Some(status_code) = set_status.and_then(|s| StatusCode::from_u16(s).ok()) {
            ctx.status = status_code;
        }
        apply_header_ops(&mut ctx.headers, ops);
    }

    Ok(DeviceResult::Continue)
}

pub(crate) fn block_403(request_id: Option<&str>) -> ResponseCtx {
    ResponseCtx::new(
        request_id.map(str::to_owned),
        StatusCode::FORBIDDEN,
        HeaderMap::new(),
        b"Blocked by device".to_vec(),
    )
}

pub(crate) fn block_503(request_id: Option<&str>) -> ResponseCtx {
    ResponseCtx::new(
        request_id.map(str::to_owned),
        StatusCode::SERVICE_UNAVAILABLE,
        HeaderMap::new(),
        b"Service unavailable".to_vec(),
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution::ctx::RequestId;
    use crate::execution::device::wasm::bindings::snakeway::device::types::SyntheticResponse;

    fn ctx_with_id(id: &str) -> RequestCtx {
        let mut ctx = RequestCtx::empty();
        ctx.extensions.insert(RequestId::from(id));
        ctx
    }

    fn response_of(result: anyhow::Result<DeviceResult>) -> ResponseCtx {
        match result {
            Ok(DeviceResult::Respond(response)) => response,
            _ => panic!("expected the device to respond"),
        }
    }

    #[test]
    fn a_blocked_request_carries_the_request_id_into_the_response() {
        // Arrange
        let mut ctx = ctx_with_id("req-1");
        let result = policy::RequestResult {
            action: Action::Block,
            patch: None,
        };

        // Act
        let response = response_of(apply_request_result(&mut ctx, result));

        // Assert
        assert_eq!(response.request_id.as_deref(), Some("req-1"));
        assert_eq!(response.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn a_synthetic_response_carries_the_request_id_and_its_status() {
        // Arrange
        let mut ctx = ctx_with_id("req-1");
        let result = policy::RequestResult {
            action: Action::Respond(SyntheticResponse {
                status: 418,
                headers: vec![],
                body: b"teapot".to_vec(),
            }),
            patch: None,
        };

        // Act
        let response = response_of(apply_request_result(&mut ctx, result));

        // Assert
        assert_eq!(response.request_id.as_deref(), Some("req-1"));
        assert_eq!(response.status, StatusCode::IM_A_TEAPOT);
    }

    #[test]
    fn a_blocked_response_keeps_its_request_id() {
        // Arrange
        let mut ctx = ResponseCtx::new(
            Some("req-2".to_string()),
            StatusCode::OK,
            HeaderMap::new(),
            Vec::new(),
        );
        let result = policy::ResponseResult {
            action: Action::Block,
            patch: None,
        };

        // Act
        let response = response_of(apply_response_result(&mut ctx, result));

        // Assert
        assert_eq!(response.request_id.as_deref(), Some("req-2"));
        assert_eq!(response.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn a_blocked_body_carries_the_request_id_into_the_response() {
        // Arrange
        let mut chunk = Some(Bytes::from_static(b"payload"));
        let result = policy::BodyResult {
            action: BodyAction::Block,
        };

        // Act
        let response = response_of(apply_body_result(Some("req-3"), &mut chunk, result));

        // Assert
        assert_eq!(response.request_id.as_deref(), Some("req-3"));
        assert_eq!(response.status, StatusCode::FORBIDDEN);
    }

    #[test]
    fn a_passed_through_body_does_not_allocate() {
        // Arrange
        let mut chunk = Some(Bytes::from_static(b"payload"));
        let result = policy::BodyResult {
            action: BodyAction::Passthrough,
        };

        // Act
        let (outcome, allocations) = crate::alloc_count::count_allocations(|| {
            apply_body_result(Some("req-3"), &mut chunk, result)
        });

        // Assert
        assert_eq!(allocations, 0);
        assert!(matches!(outcome, Ok(DeviceResult::Continue)));
    }

    #[test]
    fn a_request_that_continues_does_not_allocate() {
        // Arrange
        let mut ctx = ctx_with_id("req-1");
        let result = policy::RequestResult {
            action: Action::Continue,
            patch: None,
        };

        // Act
        let (outcome, allocations) =
            crate::alloc_count::count_allocations(|| apply_request_result(&mut ctx, result));

        // Assert
        assert_eq!(allocations, 0);
        assert!(matches!(outcome, Ok(DeviceResult::Continue)));
    }
}
