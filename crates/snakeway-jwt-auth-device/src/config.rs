use crate::bindings::host;
use crate::jwt_token_handling::AuthError;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;

pub(crate) struct AuthConfig {
    pub(crate) secret: Vec<u8>,
    pub(crate) issuer: String,
    pub(crate) audience: String,
    pub(crate) user_id_claim: String,
    pub(crate) tenant_id_claim: Option<String>,
    pub(crate) public_paths: Vec<String>,
}

impl AuthConfig {
    pub(crate) fn from_host() -> Result<Self, AuthError> {
        let secret_b64 = host::config_get("secret")
            .ok_or(AuthError::Config("missing required config key: secret"))?;

        let secret = BASE64_STANDARD
            .decode(secret_b64.as_bytes())
            .map_err(|_| AuthError::Config("secret is not valid base64"))?;

        let issuer = host::config_get("issuer")
            .ok_or(AuthError::Config("missing required config key: issuer"))?;

        let audience = host::config_get("audience")
            .ok_or(AuthError::Config("missing required config key: audience"))?;

        let user_id_claim = host::config_get("user_id_claim").unwrap_or_else(|| "sub".to_string());

        let tenant_id_claim = host::config_get("tenant_id_claim");

        let public_paths = host::config_get("public_paths")
            .map(|s| s.split(',').map(|p| p.trim().to_string()).collect())
            .unwrap_or_default();

        Ok(Self {
            secret,
            issuer,
            audience,
            user_id_claim,
            tenant_id_claim,
            public_paths,
        })
    }
}
