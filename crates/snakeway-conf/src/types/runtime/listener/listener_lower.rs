use crate::types::{BindAdminSpec, BindSpec};

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
            redirect: None,
            connection_filter,
            connection_rate_limiting_filter: spec.connection_rate_limiting_filter.map(Into::into),
        })
    }

    pub fn from_bind_admin(name: &str, spec: BindAdminSpec) -> Result<Self, String> {
        let addr = spec.resolve().map_err(|err| err.to_string())?;
        let tls = TlsTerminationConfig::try_from(spec.tls).map_err(|err| err.to_string())?;
        Ok(Self {
            name: name.to_string(),
            addr: addr.to_string(),
            tls_termination: Some(tls),
            enable_http2: false,
            enable_admin: true,
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
        let spec = BindAdminSpec {
            origin: Default::default(),
            interface: BindInterfaceInput::Keyword("loopback".to_string()),
            port: 9090,
            tls: Default::default(),
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
}
