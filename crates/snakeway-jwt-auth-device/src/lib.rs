//! # JWT Auth Gateway (Snakeway WASM Device)
//!
//! Validates JWT bearer tokens on incoming requests, extracts claims into
//! upstream headers, and returns synthetic responses on auth failure.
//!
//! ## Config (HCL)
//!
//! ```hcl
//! wasm_devices = [
//!   {
//!     name        = "jwt-auth"
//!     enable      = true
//!     path        = "/etc/snakeway/devices/jwt_auth_device.wasm"
//!     fail_policy = "closed"
//!     timeout_ms  = 5
//!
//!     config = {
//!       # HMAC-SHA256 shared secret (base64-encoded).
//!       secret   = "c2VjcmV0"
//!
//!       # Expected issuer claim. Tokens with a different `iss` are rejected.
//!       issuer   = "https://auth.example.com"
//!
//!       # Expected audience claim. Tokens with a different `aud` are rejected.
//!       audience = "https://api.example.com"
//!
//!       # Claim mapped to the X-User-Id upstream header. Default: "sub".
//!       user_id_claim = "sub"
//!
//!       # Claim mapped to the X-Tenant-Id upstream header. Optional.
//!       # Omit to skip tenant header injection.
//!       tenant_id_claim = "tenant_id"
//!
//!       # Comma-separated paths that bypass authentication entirely.
//!       # Supports exact matches only.
//!       public_paths = "/health,/ready,/.well-known/openid-configuration"
//!     }
//!   }
//! ]
//! ```
//!
//! ## Behavior
//!
//! **on-request:** Validates the JWT and injects upstream headers on success.
//!   - 401 if no `Authorization: Bearer <token>` header is present.
//!   - 401 if the token signature is invalid.
//!   - 401 if `iss` or `aud` claims do not match config.
//!   - 401 if the token is expired (`exp`) or not yet valid (`nbf`).
//!   - On success: continues with a patch that adds `X-User-Id` (and optionally
//!     `X-Tenant-Id`) and removes the `Authorization` header before proxying.
//!   - Paths listed in `public_paths` bypass validation entirely.
//!
//! **All other hooks:** Passthrough (no-op).

#[cfg(target_arch = "wasm32")]
mod config;
#[cfg(target_arch = "wasm32")]
mod jwt_auth_device;

pub(crate) mod token_validation;
pub(crate) mod types;

#[cfg(target_arch = "wasm32")]
use wit_bindgen::generate;

#[cfg(target_arch = "wasm32")]
generate!({
    path: "../snakeway-wit/wit/",
    world: "device",
});

#[cfg(target_arch = "wasm32")]
pub(crate) mod bindings {
    pub(crate) use crate::exports::snakeway::device::policy::Guest;
    pub(crate) use crate::snakeway::device::{host, types};
}

#[cfg(target_arch = "wasm32")]
use jwt_auth_device::JwtAuthDevice;
#[cfg(target_arch = "wasm32")]
export!(JwtAuthDevice);
