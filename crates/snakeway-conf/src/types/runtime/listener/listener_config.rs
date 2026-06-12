use crate::types::{
    AdminAuthConfig, BearerAuthConfig, BindAdminSpec, BindSpec, ConnectionRateLimitingFilterConfig,
    NetworkConnectionFilterConfig, TlsTerminationConfig,
};
use confval::provenance::{Lower, Report};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct ListenerConfig {
    pub name: String,

    pub addr: String,

    pub tls_termination: Option<TlsTerminationConfig>,

    pub enable_http2: bool,

    pub enable_admin: bool,

    #[serde(default)]
    pub admin_auth: Option<AdminAuthConfig>,

    pub redirect: Option<RedirectConfig>,

    pub connection_filter: Option<NetworkConnectionFilterConfig>,

    pub connection_rate_limiting_filter: Option<ConnectionRateLimitingFilterConfig>,
}

#[derive(Debug, Clone, PartialEq, Deserialize, Serialize)]
pub struct RedirectConfig {
    pub destination: String,
    pub response_code: u16,
}

/// Bridges span-first lowering into this module's `Result<_, String>`
/// plumbing: failures here mean validation was skipped or raced, so the
/// message is enough.
fn lower_or_message<C, S>(spec: &S) -> Result<C, String>
where
    C: Lower<S>,
{
    let mut report = Report::new();
    match C::lower(spec, &mut report) {
        Some(config) if !report.has_errors() => Ok(config),
        _ => Err(report
            .issues()
            .iter()
            .map(|issue| issue.message.clone())
            .collect::<Vec<_>>()
            .join("; ")),
    }
}

impl ListenerConfig {
    pub fn from_redirect(
        name: &str,
        from_addr: String,
        redirect_response_code: u16,
        spec: &BindSpec,
    ) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let connection_filter = spec
            .connection_filter
            .as_ref()
            .map(|filter| NetworkConnectionFilterConfig::try_from(&filter.value))
            .transpose()?;
        Ok(Self {
            name: name.to_string(),
            addr: from_addr,
            tls_termination: None,
            enable_http2: false,
            enable_admin: false,
            admin_auth: None,
            redirect: Some(RedirectConfig::new(
                addr.to_string(),
                redirect_response_code,
            )),
            connection_filter,
            connection_rate_limiting_filter: spec
                .connection_rate_limiting_filter
                .as_ref()
                .map(|filter| (&filter.value).into()),
        })
    }

    pub fn from_bind(name: &str, spec: &BindSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let maybe_tls = spec
            .tls
            .as_ref()
            .map(|tls| lower_or_message::<TlsTerminationConfig, _>(&tls.value))
            .transpose()?;
        let connection_filter = spec
            .connection_filter
            .as_ref()
            .map(|filter| NetworkConnectionFilterConfig::try_from(&filter.value))
            .transpose()?;
        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: maybe_tls,
            enable_http2: spec.enable_http2.value,
            enable_admin: false,
            admin_auth: None,
            redirect: None,
            connection_filter,
            connection_rate_limiting_filter: spec
                .connection_rate_limiting_filter
                .as_ref()
                .map(|filter| (&filter.value).into()),
        })
    }

    pub fn from_bind_admin(name: &str, spec: &BindAdminSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let tls = lower_or_message::<TlsTerminationConfig, _>(&spec.tls.value)?;

        let bearer_spec = spec
            .auth
            .as_ref()
            .and_then(|auth| auth.value.bearer.as_ref())
            .ok_or_else(|| {
                "admin auth is missing at lowering time (bug: validation should have caught this)"
                    .to_string()
            })?;
        let bearer =
            BearerAuthConfig::try_from(&bearer_spec.value).map_err(|err| err.to_string())?;
        let admin_auth = Some(AdminAuthConfig {
            bearer: Some(bearer),
        });

        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: Some(tls),
            enable_http2: false,
            enable_admin: true,
            admin_auth,
            redirect: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        })
    }
}

impl RedirectConfig {
    pub fn new(destination: String, response_code: u16) -> Self {
        Self {
            destination,
            response_code,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Located;

    fn minimal_bind() -> BindSpec {
        BindSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(8080),
            ..Default::default()
        }
    }

    #[test]
    fn from_bind_creates_listener() {
        // Arrange
        let spec = minimal_bind();

        // Act
        let config = ListenerConfig::from_bind("test-listener", &spec).unwrap();

        // Assert
        assert_eq!(config.name, "test-listener");
        assert_eq!(config.addr, "127.0.0.1:8080");
        assert!(!config.enable_admin);
        assert!(!config.enable_http2);
        assert!(config.tls_termination.is_none());
        assert!(config.redirect.is_none());
    }

    #[test]
    fn from_bind_admin_sets_admin_flag() {
        // Arrange
        use crate::types::{AdminAuthSpec, BearerAuthSpec};
        use std::io::Write;
        let mut token_file = tempfile::NamedTempFile::new().expect("tempfile");
        token_file
            .write_all(b"a9f1c38de4b67029c5d1e97f4a0ebac12d3b8ffc84e1d27a05f6cb9e83d21a04\n")
            .unwrap();

        let spec = BindAdminSpec {
            interface: Located::detached("loopback".to_string()),
            port: Located::detached(9090),
            tls: Located::detached(Default::default()),
            auth: Some(Located::detached(AdminAuthSpec {
                bearer: Some(Located::detached(BearerAuthSpec {
                    token_file: Located::detached(token_file.path().to_path_buf()),
                })),
            })),
        };

        // Act
        let config = ListenerConfig::from_bind_admin("admin-listener", &spec).unwrap();

        // Assert
        assert_eq!(config.name, "admin-listener");
        assert_eq!(config.addr, "127.0.0.1:9090");
        assert!(config.enable_admin);
        assert!(config.tls_termination.is_some());
        assert!(!config.enable_http2);
        assert!(config.redirect.is_none());
        assert!(config.connection_filter.is_none());
        assert!(config.admin_auth.is_some());
        assert!(config.admin_auth.as_ref().unwrap().bearer.is_some());
    }

    #[test]
    fn from_redirect_creates_redirect_listener() {
        // Arrange
        let spec = BindSpec {
            port: Located::detached(8443),
            ..minimal_bind()
        };

        // Act
        let config = ListenerConfig::from_redirect(
            "redirect-listener",
            "127.0.0.1:8080".to_string(),
            308,
            &spec,
        )
        .unwrap();

        // Assert
        assert_eq!(config.name, "redirect-listener");
        assert_eq!(config.addr, "127.0.0.1:8080");
        assert!(!config.enable_admin);
        assert!(config.tls_termination.is_none());
        let redirect = config.redirect.unwrap();
        assert_eq!(redirect.destination, "127.0.0.1:8443");
        assert_eq!(redirect.response_code, 308);
    }
}
