use super::RequestFilterDeviceConfig;
use crate::types::RequestFilterDeviceSpec;
use crate::validation::ConfigError;
use http::{HeaderName, Method};
use smallvec::SmallVec;
use std::time::Duration;

impl TryFrom<RequestFilterDeviceSpec> for RequestFilterDeviceConfig {
    type Error = ConfigError;

    fn try_from(spec: RequestFilterDeviceSpec) -> Result<Self, Self::Error> {
        let origin = spec.origin.clone();

        let allow_methods: SmallVec<[Method; 4]> = spec
            .allow_methods
            .into_iter()
            .map(|s| {
                Method::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidMethod {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<SmallVec<_>, _>>()?;

        let deny_methods: SmallVec<[Method; 4]> = spec
            .deny_methods
            .into_iter()
            .map(|s| {
                Method::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidMethod {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<SmallVec<_>, _>>()?;

        let deny_headers = spec
            .deny_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<SmallVec<_>, _>>()?;

        let allow_headers: SmallVec<[HeaderName; 8]> = spec
            .allow_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<SmallVec<_>, _>>()?;

        let required_headers: SmallVec<[HeaderName; 8]> = spec
            .required_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<SmallVec<_>, _>>()?;

        Ok(Self {
            enable: spec.enable,
            allow_methods,
            deny_methods,
            deny_headers,
            allow_headers,
            required_headers,
            max_header_bytes: spec.max_header_bytes,
            max_body_bytes: spec.max_body_bytes,
            max_suspicious_body_bytes: spec.max_suspicious_body_bytes,
            deny_status: spec.deny_status,
            client_body_timeout: spec.client_body_timeout_seconds.map(Duration::from_secs),
            paths: spec.paths.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::Origin;
    use std::path::PathBuf;

    fn test_origin() -> Origin {
        Origin {
            file: PathBuf::from("test.hcl"),
            section: "device.request_filter".to_string(),
            index: None,
        }
    }

    fn default_spec() -> RequestFilterDeviceSpec {
        RequestFilterDeviceSpec {
            origin: test_origin(),
            enable: true,
            allow_methods: vec![],
            deny_methods: vec![],
            deny_headers: vec![],
            allow_headers: vec![],
            required_headers: vec![],
            max_header_bytes: 16 * 1024,
            max_body_bytes: 1024 * 1024,
            max_suspicious_body_bytes: 8 * 1024,
            client_body_timeout_seconds: None,
            deny_status: None,
            paths: vec![],
        }
    }

    #[test]
    fn valid_methods_parsed() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            allow_methods: vec!["GET".to_string(), "POST".to_string()],
            ..default_spec()
        };

        // Act
        let config = RequestFilterDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.allow_methods.len(), 2);
        assert_eq!(config.allow_methods[0], Method::GET);
        assert_eq!(config.allow_methods[1], Method::POST);
    }

    #[test]
    fn invalid_method_fails() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            allow_methods: vec!["INVALID METHOD".to_string()],
            ..default_spec()
        };

        // Act
        let result = RequestFilterDeviceConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidMethod { .. }
        ));
    }

    #[test]
    fn valid_headers_parsed() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            deny_headers: vec!["x-custom".to_string()],
            ..default_spec()
        };

        // Act
        let config = RequestFilterDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.deny_headers.len(), 1);
        assert_eq!(config.deny_headers[0], HeaderName::from_static("x-custom"));
    }

    #[test]
    fn invalid_header_fails() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            deny_headers: vec!["invalid header!".to_string()],
            ..default_spec()
        };

        // Act
        let result = RequestFilterDeviceConfig::try_from(spec);

        // Assert
        assert!(result.is_err());
        assert!(matches!(
            result.unwrap_err(),
            ConfigError::InvalidHeaderName { .. }
        ));
    }

    #[test]
    fn client_body_timeout_converted() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            client_body_timeout_seconds: Some(30),
            ..default_spec()
        };

        // Act
        let config = RequestFilterDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.client_body_timeout, Some(Duration::from_secs(30)));
    }
}
