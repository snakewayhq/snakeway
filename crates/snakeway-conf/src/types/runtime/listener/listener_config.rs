use crate::resolution::ResolveError;
use crate::types::{
    AdminAuthConfig, BearerAuthConfig, BindAdminSpec, BindSpec, ConnectionRateLimitingFilterConfig,
    NetworkConnectionFilterConfig, TlsTerminationConfig,
};
use confval::provenance::{Located, Lower, Report};
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

/// Report a bind resolution failure at the interface that declared it.
fn report_resolve_error(err: &ResolveError, interface: &Located<String>, report: &mut Report) {
    report
        .error(format!("invalid bind address: {err}"))
        .at(interface.span)
        .emit();
}

/// Lower an optional connection filter. The outer `Option` is the failure
/// channel; the inner one mirrors the spec field being optional.
fn lower_connection_filter(
    spec: &BindSpec,
    report: &mut Report,
) -> Option<Option<NetworkConnectionFilterConfig>> {
    match &spec.connection_filter {
        Some(filter) => NetworkConnectionFilterConfig::lower(&filter.value, report).map(Some),
        None => Some(None),
    }
}

impl ListenerConfig {
    pub fn from_redirect(
        name: &str,
        from_addr: String,
        redirect_response_code: u16,
        spec: &BindSpec,
        report: &mut Report,
    ) -> Option<Self> {
        let addr = spec
            .resolve()
            .map_err(|err| report_resolve_error(&err, &spec.interface, report))
            .ok();
        let connection_filter = lower_connection_filter(spec, report);

        Some(Self {
            name: name.to_string(),
            addr: from_addr,
            tls_termination: None,
            enable_http2: false,
            enable_admin: false,
            admin_auth: None,
            redirect: Some(RedirectConfig::new(
                addr?.to_string(),
                redirect_response_code,
            )),
            connection_filter: connection_filter?,
            connection_rate_limiting_filter: match &spec.connection_rate_limiting_filter {
                Some(filter) => Some(ConnectionRateLimitingFilterConfig::lower(
                    &filter.value,
                    report,
                )?),
                None => None,
            },
        })
    }

    pub fn from_bind(name: &str, spec: &BindSpec, report: &mut Report) -> Option<Self> {
        let addr = spec
            .resolve()
            .map_err(|err| report_resolve_error(&err, &spec.interface, report))
            .ok();
        let maybe_tls = match &spec.tls {
            Some(tls) => TlsTerminationConfig::lower(&tls.value, report).map(Some),
            None => Some(None),
        };
        let connection_filter = lower_connection_filter(spec, report);

        Some(Self {
            name: name.to_string(),
            addr: addr?.to_string(),
            tls_termination: maybe_tls?,
            enable_http2: spec.enable_http2.value,
            enable_admin: false,
            admin_auth: None,
            redirect: None,
            connection_filter: connection_filter?,
            connection_rate_limiting_filter: match &spec.connection_rate_limiting_filter {
                Some(filter) => Some(ConnectionRateLimitingFilterConfig::lower(
                    &filter.value,
                    report,
                )?),
                None => None,
            },
        })
    }

    pub fn from_bind_admin(name: &str, spec: &BindAdminSpec, report: &mut Report) -> Option<Self> {
        let addr = spec
            .resolve()
            .map_err(|err| report_resolve_error(&err, &spec.interface, report))
            .ok();
        let tls = TlsTerminationConfig::lower(&spec.tls.value, report);

        let bearer = match spec
            .auth
            .as_ref()
            .and_then(|auth| auth.value.bearer.as_ref())
        {
            Some(bearer_spec) => match BearerAuthConfig::try_from(&bearer_spec.value) {
                Ok(bearer) => Some(bearer),
                Err(err) => {
                    report
                        .error(err.to_string())
                        .at(bearer_spec.value.token_file.span)
                        .emit();
                    None
                }
            },
            None => {
                report
                    .error(
                        "admin auth is missing at lowering time \
                         (bug: validation should have caught this)",
                    )
                    .at(spec.interface.span)
                    .emit();
                None
            }
        };

        Some(Self {
            name: name.to_string(),
            addr: addr?.to_string(),
            tls_termination: Some(tls?),
            enable_http2: false,
            enable_admin: true,
            admin_auth: Some(AdminAuthConfig {
                bearer: Some(bearer?),
            }),
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
        let config = ListenerConfig::from_bind("test-listener", &spec, &mut Report::new()).unwrap();

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
        let config =
            ListenerConfig::from_bind_admin("admin-listener", &spec, &mut Report::new()).unwrap();

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
            &mut Report::new(),
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
