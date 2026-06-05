use crate::bindings::{Guest, host, types};
use crate::config::AuthConfig;
use crate::token_validation::{AuthError, ValidatedToken, validate_token};
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
            return RequestResult {
                action: Action::Continue,
                patch: None,
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

        match validate_token(raw_token, &config) {
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
    if trimmed.len() > 7 && trimmed[..7].eq_ignore_ascii_case("bearer ") {
        Some(trimmed[7..].trim())
    } else {
        None
    }
}

fn error_body(error: &str) -> Vec<u8> {
    let escaped = error.replace('\\', "\\\\").replace('"', "\\\"");
    format!(r#"{{"error":"{escaped}"}}"#).into_bytes()
}

fn error_response(err: &AuthError) -> Action {
    Action::Respond(SyntheticResponse {
        status: err.status(),
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
        body: error_body(err.message()),
    })
}

fn success_patch(token: &ValidatedToken, config: &AuthConfig) -> Option<RequestPatch> {
    let mut ops = Vec::new();

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

    ops.push(HeaderOp::Remove("authorization".to_string()));

    Some(RequestPatch {
        set_route_path: None,
        set_upstream_path: None,
        ops,
    })
}
