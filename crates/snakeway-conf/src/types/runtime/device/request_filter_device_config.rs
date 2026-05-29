use crate::types::RequestFilterDeviceSpec;
use crate::validation::ConfigError;
use http::{HeaderName, Method};
use serde::{Deserialize, Serialize};
use smallvec::SmallVec;
use std::time::Duration;

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFilterDeviceConfig {
    pub enable: bool,
    #[serde(with = "serde_method_vec")]
    pub allow_methods: SmallVec<[Method; 4]>,
    #[serde(with = "serde_method_vec")]
    pub deny_methods: SmallVec<[Method; 4]>,
    #[serde(with = "serde_header_name_vec")]
    pub deny_headers: SmallVec<[HeaderName; 8]>,
    #[serde(with = "serde_header_name_vec")]
    pub allow_headers: SmallVec<[HeaderName; 8]>,
    #[serde(with = "serde_header_name_vec")]
    pub required_headers: SmallVec<[HeaderName; 8]>,
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub max_suspicious_body_bytes: usize,
    pub deny_status: Option<u16>,
    pub client_body_timeout: Option<Duration>,
    pub paths: SmallVec<[String; 4]>,
}

// Serialization helpers
mod serde_header_name_vec {
    use http::HeaderName;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use smallvec::SmallVec;

    pub fn serialize<S>(headers: &[HeaderName], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&str> = headers.iter().map(|h| h.as_str()).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SmallVec<[HeaderName; 8]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strings: Vec<String> = Vec::deserialize(deserializer)?;
        strings
            .into_iter()
            .map(|s| HeaderName::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom))
            .collect()
    }
}

mod serde_method_vec {
    use http::Method;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};
    use smallvec::SmallVec;

    pub fn serialize<S>(methods: &[Method], serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&str> = methods.iter().map(|m| m.as_str()).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<SmallVec<[Method; 4]>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let strings: Vec<String> = Vec::deserialize(deserializer)?;
        strings
            .into_iter()
            .map(|s| Method::from_bytes(s.as_bytes()).map_err(serde::de::Error::custom))
            .collect()
    }
}

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
            max_header_bytes: spec.max_header_bytes as usize,
            max_body_bytes: spec.max_body_bytes as usize,
            max_suspicious_body_bytes: spec.max_suspicious_body_bytes as usize,
            deny_status: spec.deny_status.map(|v| v as u16),
            client_body_timeout: spec
                .client_body_timeout_seconds
                .map(|v| Duration::from_secs(v as u64)),
            paths: spec.paths.into_iter().collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::types::HclOrigin;
    use std::path::PathBuf;

    fn test_origin() -> HclOrigin {
        HclOrigin {
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
