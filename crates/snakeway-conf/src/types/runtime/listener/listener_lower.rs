use crate::types::{AdminAuthConfig, BearerAuthConfig, BindAdminSpec, BindSpec};

use super::{ListenerConfig, RedirectConfig, TlsTerminationConfig};

impl ListenerConfig {
    pub fn from_redirect(
        name: &str,
        from_addr: String,
        redirect_response_code: u16,
        spec: BindSpec,
    ) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let connection_filter = spec.connection_filter.map(TryInto::try_into).transpose()?;
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
            connection_rate_limiting_filter: spec.connection_rate_limiting_filter.map(Into::into),
        })
    }

    pub fn from_bind(name: &str, spec: BindSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let maybe_tls = if let Some(tls) = spec.tls {
            Some(TlsTerminationConfig::try_from(tls).map_err(|err| err.to_string())?)
        } else {
            None
        };
        let connection_filter = spec.connection_filter.map(TryInto::try_into).transpose()?;
        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: maybe_tls,
            enable_http2: spec.enable_http2,
            enable_admin: false,
            admin_auth: None,
            redirect: None,
            connection_filter,
            connection_rate_limiting_filter: spec.connection_rate_limiting_filter.map(Into::into),
        })
    }

    pub fn from_bind_admin(name: &str, spec: BindAdminSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let tls = TlsTerminationConfig::try_from(spec.tls).map_err(|err| err.to_string())?;

        // Validation guarantees bearer is Some and parses cleanly. Any error
        // here means validation was skipped or the token file was mutated
        // after validation; surface it as a lowering error rather than
        // panicking.
        let bearer_spec = spec.auth.bearer.ok_or_else(|| {
            "admin auth is missing at lowering time (bug: validation should have caught this)"
                .to_string()
        })?;
        let bearer = BearerAuthConfig::try_from(bearer_spec).map_err(|err| err.to_string())?;
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
    use crate::types::BindInterfaceInput;

    #[test]
    fn from_bind_creates_listener() {
        // Arrange
        let spec = BindSpec {
            origin: Default::default(),
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            tls: None,
            enable_http2: false,
            redirect_http_to_https: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        };

        // Act
        let config = ListenerConfig::from_bind("test-listener", spec).unwrap();

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
            origin: Default::default(),
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9090,
            tls: Default::default(),
            auth: AdminAuthSpec {
                bearer: Some(BearerAuthSpec {
                    token_file: token_file.path().to_path_buf(),
                    origin: Default::default(),
                }),
                origin: Default::default(),
            },
        };

        // Act
        let config = ListenerConfig::from_bind_admin("admin-listener", spec).unwrap();

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
            origin: Default::default(),
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8443,
            tls: None,
            enable_http2: false,
            redirect_http_to_https: None,
            connection_filter: None,
            connection_rate_limiting_filter: None,
        };

        // Act
        let config = ListenerConfig::from_redirect(
            "redirect-listener",
            "127.0.0.1:8080".to_string(),
            308,
            spec,
        )
        .unwrap();

        // Assert
        assert_eq!(config.name, "redirect-listener");
        assert_eq!(config.addr, "127.0.0.1:8080");
        assert!(!config.enable_admin);
        assert!(!config.enable_http2);
        assert!(config.tls_termination.is_none());

        let redirect = config.redirect.expect("redirect should be set");
        assert_eq!(redirect.destination, "127.0.0.1:8443");
        assert_eq!(redirect.response_code, 308);
    }

    #[test]
    fn from_bind_invalid_interface_returns_error() {
        // Arrange
        let spec = BindSpec {
            interface: BindInterfaceInput::Keyword("bad-interface".to_string()),
            port: 8080,
            ..Default::default()
        };

        // Act
        let result = ListenerConfig::from_bind("test", spec);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn from_bind_admin_invalid_interface_returns_error() {
        // Arrange
        let spec = BindAdminSpec {
            interface: BindInterfaceInput::Keyword("bad-interface".to_string()),
            port: 9090,
            ..Default::default()
        };

        // Act
        let result = ListenerConfig::from_bind_admin("test", spec);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn from_redirect_invalid_interface_returns_error() {
        // Arrange
        let spec = BindSpec {
            interface: BindInterfaceInput::Keyword("bad-interface".to_string()),
            port: 8443,
            ..Default::default()
        };

        // Act
        let result = ListenerConfig::from_redirect("test", "0.0.0.0:80".to_string(), 308, spec);

        // Assert
        assert!(result.is_err());
    }

    #[test]
    fn from_bind_invalid_connection_filter_cidr_returns_error() {
        // Arrange
        use crate::types::{CidrSpec, IpFamilySpec, NetworkConnectionFilterSpec};
        let spec = BindSpec {
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 8080,
            connection_filter: Some(NetworkConnectionFilterSpec {
                cidr: CidrSpec {
                    allow: vec!["not-a-cidr".to_string()],
                    deny: vec![],
                },
                ip_family: IpFamilySpec {
                    ipv4: true,
                    ipv6: false,
                },
                ..Default::default()
            }),
            ..Default::default()
        };

        // Act
        let result = ListenerConfig::from_bind("test", spec);

        // Assert
        assert!(result.is_err());
    }
}
