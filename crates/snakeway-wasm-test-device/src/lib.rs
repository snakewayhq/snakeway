use wit_bindgen::generate;

generate!({
    path: "../snakeway-wit/wit/",
    world: "device",
});

use exports::snakeway::device::policy::Guest;
use snakeway::device::host;
use snakeway::device::types::{
    Action, BodyAction, BodyChunk, BodyResult, Header, HeaderOp, Request, RequestPatch,
    RequestResult, Response, ResponsePatch, ResponseResult, SyntheticResponse,
};

struct TestDevice;

impl Guest for TestDevice {
    fn on_request(req: Request) -> RequestResult {
        host::log(2, "on_request called");
        host::metric_increment("requests_seen", 1);

        match req.route_path.as_str() {
            "/block" => RequestResult {
                action: Action::Block,
                patch: None,
            },

            "/synthetic" => RequestResult {
                action: Action::Respond(SyntheticResponse {
                    status: 299,
                    headers: vec![Header {
                        name: "content-type".to_string(),
                        value: "text/plain".to_string(),
                    }],
                    body: b"synthetic-ok".to_vec(),
                }),
                patch: None,
            },

            "/rewrite" => RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: Some("/api".to_string()),
                    set_upstream_path: None,
                    ops: vec![],
                }),
            },

            "/set-header" => RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: None,
                    set_upstream_path: None,
                    ops: vec![HeaderOp::Set(Header {
                        name: "x-wasm-test".to_string(),
                        value: "injected".to_string(),
                    })],
                }),
            },

            "/append-header" => RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: None,
                    set_upstream_path: None,
                    ops: vec![
                        HeaderOp::Append(Header {
                            name: "x-multi".to_string(),
                            value: "a".to_string(),
                        }),
                        HeaderOp::Append(Header {
                            name: "x-multi".to_string(),
                            value: "b".to_string(),
                        }),
                    ],
                }),
            },

            "/remove-header" => RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: None,
                    set_upstream_path: None,
                    ops: vec![HeaderOp::Remove("x-to-remove".to_string())],
                }),
            },

            "/config-echo" => {
                let value = host::config_get("echo_value").unwrap_or_default();
                RequestResult {
                    action: Action::Respond(SyntheticResponse {
                        status: 200,
                        headers: vec![Header {
                            name: "content-type".to_string(),
                            value: "text/plain".to_string(),
                        }],
                        body: value.into_bytes(),
                    }),
                    patch: None,
                }
            }

            _ => RequestResult {
                action: Action::Continue,
                patch: None,
            },
        }
    }

    fn on_stream_request_body(_req: Request, chunk: Option<BodyChunk>) -> BodyResult {
        let mode = host::config_get("mode").unwrap_or_default();

        if let Some(ref c) = chunk
            && c.data.windows(10).any(|w| w == b"BLOCK_BODY")
        {
            return BodyResult {
                action: BodyAction::Block,
            };
        }

        match mode.as_str() {
            "replace" => BodyResult {
                action: BodyAction::Replace(b"replaced".to_vec()),
            },
            "drop" => BodyResult {
                action: BodyAction::Drop,
            },
            _ => BodyResult {
                action: BodyAction::Passthrough,
            },
        }
    }

    fn before_proxy(req: Request) -> RequestResult {
        let mode = host::config_get("mode").unwrap_or_default();

        if mode == "inject" {
            return RequestResult {
                action: Action::Continue,
                patch: Some(RequestPatch {
                    set_route_path: None,
                    set_upstream_path: None,
                    ops: vec![HeaderOp::Set(Header {
                        name: "x-before-proxy".to_string(),
                        value: "injected".to_string(),
                    })],
                }),
            };
        }

        let _ = req;
        RequestResult {
            action: Action::Continue,
            patch: None,
        }
    }

    fn after_proxy(resp: Response) -> ResponseResult {
        let mode = host::config_get("mode").unwrap_or_default();

        if mode == "set-status" {
            return ResponseResult {
                action: Action::Continue,
                patch: Some(ResponsePatch {
                    set_status: Some(299),
                    ops: vec![],
                }),
            };
        }

        let _ = resp;
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

    fn on_response(resp: Response) -> ResponseResult {
        let mode = host::config_get("mode").unwrap_or_default();

        if mode == "tag-response" {
            return ResponseResult {
                action: Action::Continue,
                patch: Some(ResponsePatch {
                    set_status: None,
                    ops: vec![HeaderOp::Set(Header {
                        name: "x-wasm-response".to_string(),
                        value: "tagged".to_string(),
                    })],
                }),
            };
        }

        let _ = resp;
        ResponseResult {
            action: Action::Continue,
            patch: None,
        }
    }
}

export!(TestDevice);
