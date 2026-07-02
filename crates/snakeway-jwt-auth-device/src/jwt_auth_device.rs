use crate::bindings::{Guest, host, types};
use crate::token_validation::{AuthError, validate_token};
use crate::types::{AuthConfig, ValidatedToken};
use types::{
    Action, BodyAction, BodyChunk, BodyResult, Header, HeaderOp, Request, RequestPatch,
    RequestResult, Response, ResponseResult, SyntheticResponse,
};

pub(crate) struct JwtAuthDevice;

impl Guest for JwtAuthDevice {
    fn on_request(req: Request) -> RequestResult {
        let config = match AuthConfig::from_host() {
            Ok(c) => c,
            Err(e) => {
                host::log(4, &format!("jwt-auth config error: {}", e.log_message()));
                host::metric_increment("auth_config_errors", 1);
                return RequestResult {
                    action: error_response(&e),
                    patch: None,
                };
            }
        };

        if config.public_paths.iter().any(|p| p == &req.route_path) {
            host::log(0, &format!("public path bypass: {}", req.route_path));
            // Even on a bypass, strip client-supplied identity headers so a
            // request to a public path cannot spoof X-User-Id / X-Tenant-Id to
            // an upstream that trusts them.
            return RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: None,
                    set_upstream_path: None,
                    ops: vec![
                        HeaderOp::Remove("x-user-id".to_string()),
                        HeaderOp::Remove("x-tenant-id".to_string()),
                    ],
                }),
            };
        }

        let auth_header = req
            .headers
            .iter()
            .find(|h| h.name.eq_ignore_ascii_case("authorization"));

        let raw_token = match auth_header.and_then(|h| extract_bearer_token(&h.value)) {
            Some(t) => t,
            None => {
                host::log(
                    2,
                    &format!("no bearer token: {} {}", req.method, req.route_path),
                );
                host::metric_increment("auth_rejected", 1);
                return RequestResult {
                    action: error_response(&AuthError::NoToken),
                    patch: None,
                };
            }
        };

        let now = host::epoch_secs();
        match validate_token(raw_token, &config, now) {
            Ok(token) => {
                let user_id = token
                    .claims
                    .get_claim(&config.user_id_claim)
                    .unwrap_or_else(|| "unknown".to_string());

                host::log(
                    1,
                    &format!(
                        "auth ok: user={} {} {}",
                        user_id, req.method, req.route_path
                    ),
                );
                host::metric_increment("auth_accepted", 1);

                RequestResult {
                    action: Action::Continue,
                    patch: success_patch(&token, &config),
                }
            }
            Err(e) => {
                host::log(
                    3,
                    &format!(
                        "auth rejected: {} — {} {}",
                        e.log_message(),
                        req.method,
                        req.route_path
                    ),
                );
                host::metric_increment("auth_rejected", 1);

                RequestResult {
                    action: error_response(&e),
                    patch: None,
                }
            }
        }
    }

    fn on_stream_request_body(_req: Request, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult {
            action: BodyAction::Passthrough,
        }
    }

    fn before_proxy(_req: Request) -> RequestResult {
        RequestResult {
            action: Action::Continue,
            patch: None,
        }
    }

    fn after_proxy(_resp: Response) -> ResponseResult {
        ResponseResult {
            action: Action::Continue,
            patch: None,
        }
    }

    fn on_stream_response_body(_resp: Response, _chunk: Option<BodyChunk>) -> BodyResult {
        BodyResult {
            action: BodyAction::Passthrough,
        }
    }

    fn on_response(_resp: Response) -> ResponseResult {
        ResponseResult {
            action: Action::Continue,
            patch: None,
        }
    }
}

//-----------------------------------------------------------------------------
// Helpers
//-----------------------------------------------------------------------------

fn extract_bearer_token(auth_value: &str) -> Option<&str> {
    let trimmed = auth_value.trim();
    // Compare on bytes, not a str range index: a `str` byte-range slice panics if
    // it splits a multi-byte UTF-8 character. Once the ASCII "bearer " prefix
    // matches, byte index 7 is a valid char boundary, so `trimmed[7..]` is safe.
    if trimmed.len() > 7 && trimmed.as_bytes()[..7].eq_ignore_ascii_case(b"bearer ") {
        Some(trimmed[7..].trim())
    } else {
        None
    }
}

/// Return a generic body so the client cannot use the specific failure reason
/// as an oracle (which of iss/aud/exp failed) or learn that the auth service is
/// misconfigured.
/// The specific reason is recorded server-side via log_message.
fn error_response(err: &AuthError) -> Action {
    let status = err.status();
    let body = if status == 500 {
        br#"{"error":"internal error"}"#.to_vec()
    } else {
        br#"{"error":"unauthorized"}"#.to_vec()
    };
    Action::Respond(SyntheticResponse {
        status,
        headers: vec![
            Header {
                name: "content-type".to_string(),
                value: "application/json".to_string(),
            },
            Header {
                name: "cache-control".to_string(),
                value: "no-store".to_string(),
            },
        ],
        body,
    })
}

fn success_patch(token: &ValidatedToken, config: &AuthConfig) -> Option<RequestPatch> {
    // Always neutralize client-supplied identity headers and the bearer token
    // before asserting our own. The removes run unconditionally so a client can
    // never spoof identity by sending these headers (e.g. when a configured
    // claim like tenant is absent and no Set is emitted below).
    let mut ops = vec![
        HeaderOp::Remove("x-user-id".to_string()),
        HeaderOp::Remove("x-tenant-id".to_string()),
        HeaderOp::Remove("authorization".to_string()),
    ];

    if let Some(user_id) = token.claims.get_claim(&config.user_id_claim) {
        ops.push(HeaderOp::Set(Header {
            name: "x-user-id".to_string(),
            value: user_id,
        }));
    }

    if let Some(ref tenant_claim) = config.tenant_id_claim
        && let Some(tenant_id) = token.claims.get_claim(tenant_claim)
    {
        ops.push(HeaderOp::Set(Header {
            name: "x-tenant-id".to_string(),
            value: tenant_id,
        }));
    }

    Some(RequestPatch {
        set_route_path: None,
        set_upstream_path: None,
        ops,
    })
}
