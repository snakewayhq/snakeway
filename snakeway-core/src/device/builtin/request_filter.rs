use crate::conf::types::RequestFilterDeviceConfig;
use crate::ctx::{RequestCtx, ResponseCtx};
use crate::device::core::{Device, DeviceResult};
use http::{HeaderName, Method, StatusCode};
use smallvec::SmallVec;

/// RequestFilter validates incoming HTTP requests against various rules.
///
/// This struct uses `SmallVec` for storing lists of HTTP methods and headers.
/// SmallVec is a special list type that stores a few items directly inside itself
/// (like a small backpack), and only allocates extra memory when you need more space.
///
/// For example, `SmallVec<[Method; 4]>` can hold up to 4 HTTP methods without needing
/// to allocate memory separately. Since most filters only check a few methods
/// (like GET, POST, PUT, DELETE), this saves memory and makes the code faster.
/// The same applies to headers - most filters only care about a handful of headers,
/// so storing 8 directly is usually enough.
///
/// Think of it like this: instead of always using a big warehouse (heap allocation)
/// to store a few items, we use a small shelf (stack storage) first, and only rent
/// warehouse space when we really need it.
#[derive(Debug)]
pub struct RequestFilterDevice {
    pub allow_methods: SmallVec<[Method; 4]>,
    pub deny_methods: SmallVec<[Method; 4]>,
    pub deny_headers: SmallVec<[HeaderName; 8]>,
    pub allow_headers: SmallVec<[HeaderName; 8]>,
    pub required_headers: SmallVec<[HeaderName; 8]>,
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub deny_status: Option<u16>,
}

impl RequestFilterDevice {
    pub fn from_config(cfg: RequestFilterDeviceConfig) -> anyhow::Result<Self> {
        Ok(Self {
            allow_methods: cfg.allow_methods.into_iter().collect(),
            deny_methods: cfg.deny_methods.into_iter().collect(),
            deny_headers: cfg.deny_headers.into_iter().collect(),
            allow_headers: cfg.allow_headers.into_iter().collect(),
            required_headers: cfg.required_headers.into_iter().collect(),
            max_header_bytes: cfg.max_header_bytes,
            max_body_bytes: cfg.max_body_bytes,
            deny_status: cfg.deny_status,
        })
    }

    fn deny(
        &self,
        ctx: &RequestCtx,
        default_status: StatusCode,
        reason: &'static str,
    ) -> DeviceResult {
        let status = match self.deny_status {
            Some(status) => StatusCode::from_u16(status).unwrap_or(default_status),
            None => default_status,
        };

        DeviceResult::Respond(ResponseCtx::new(
            ctx.request_id(),
            status,
            Default::default(),
            reason.as_bytes().to_vec(),
        ))
    }
}

impl Device for RequestFilterDevice {
    /// RequestFilter is a request-only gate by design
    /// It should only act on ctx.normalized_request
    ///
    /// Matching order...
    /// 1. Size limits
    /// 2. Methods gates
    /// 3. Header gates
    fn on_request(&self, ctx: &mut RequestCtx) -> DeviceResult {
        //---------------------------------------------------------------------
        // 1. Size limits
        //---------------------------------------------------------------------
        if ctx.body.len() > self.max_body_bytes {
            return self.deny(ctx, StatusCode::PAYLOAD_TOO_LARGE, "Request body too large");
        }

        if !ctx.headers.is_empty() {
            let header_bytes: usize = ctx
                .headers
                .iter()
                .map(|(k, v)| {
                    k.as_str().len()
                    + 2 // ": "
                    + v.as_bytes().len()
                    + 2 // "\r\n"
                })
                .sum();
            if header_bytes > self.max_header_bytes {
                return self.deny(
                    ctx,
                    StatusCode::REQUEST_HEADER_FIELDS_TOO_LARGE,
                    "Request headers too large",
                );
            }
        }

        //---------------------------------------------------------------------
        // 2. Method gates
        //---------------------------------------------------------------------
        let method = ctx.method();

        if self.deny_methods.contains(method)
            || (!self.allow_methods.is_empty() && !self.allow_methods.contains(method))
        {
            // deny
            return self.deny(ctx, StatusCode::METHOD_NOT_ALLOWED, "Method forbidden");
        }

        //---------------------------------------------------------------------
        // 3. Header gates
        //---------------------------------------------------------------------
        // Check if any deny list headers are present.
        if self
            .deny_headers
            .iter()
            .any(|h| ctx.headers.contains_key(h))
        {
            // Forbidden header.
            return self.deny(ctx, StatusCode::FORBIDDEN, "Header denied");
        }

        // Required headers.
        if !self
            .required_headers
            .iter()
            .all(|h| ctx.headers.contains_key(h))
        {
            // Missing one or more required headers.
            return self.deny(ctx, StatusCode::BAD_REQUEST, "Required header missing");
        }

        // Allowlist headers (only if non-empty)
        if !self.allow_headers.is_empty()
            && ctx.headers.keys().any(|h| !self.allow_headers.contains(h))
        {
            // Forbidden header.
            return self.deny(ctx, StatusCode::FORBIDDEN, "Header not allowed");
        }

        // Return normally - no gates tripped.
        DeviceResult::Continue
    }

    fn before_proxy(&self, _: &mut RequestCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn after_proxy(&self, _: &mut ResponseCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn on_response(&self, _: &mut ResponseCtx) -> DeviceResult {
        // RequestFilter is a request-only gate by design
        DeviceResult::Continue
    }

    fn on_error(&self, _: &crate::device::core::errors::DeviceError) {}
}
