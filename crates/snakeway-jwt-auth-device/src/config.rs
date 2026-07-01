use crate::bindings::host;
use crate::token_validation::AuthError;
use crate::types::AuthConfig;
use base64::Engine;
use base64::prelude::BASE64_STANDARD;

impl AuthConfig {
    pub(crate) fn from_host() -> Result<Self, AuthError> {
        let secret_b64 = host::config_get("secret")
            .ok_or(AuthError::Config("missing required config key: secret"))?;

        let secret = BASE64_STANDARD
            .decode(secret_b64.as_bytes())
            .map_err(|_| AuthError::Config("secret is not valid base64"))?;
        crate::token_validation::validate_secret(&secret)?;

        let issuer = host::config_get("issuer")
            .ok_or(AuthError::Config("missing required config key: issuer"))?;

        let audience = host::config_get("audience")
            .ok_or(AuthError::Config("missing required config key: audience"))?;

        let user_id_claim = host::config_get("user_id_claim").unwrap_or_else(|| "sub".to_string());

        let tenant_id_claim = host::config_get("tenant_id_claim");

        let public_paths = host::config_get("public_paths")
            .map(|s| {
                s.split(',')
                    .map(|p| p.trim().to_string())
                    .filter(|p| !p.is_empty())
                    .collect()
            })
            .unwrap_or_default();

        let token_type = host::config_get("token_type");

        let clock_skew_leeway_seconds = crate::token_validation::parse_leeway_seconds(
            host::config_get("clock_skew_leeway_seconds").as_deref(),
        )?;

        Ok(Self {
            secret,
            issuer,
            audience,
            user_id_claim,
            tenant_id_claim,
            public_paths,
            token_type,
            clock_skew_leeway_seconds,
        })
    }
}
