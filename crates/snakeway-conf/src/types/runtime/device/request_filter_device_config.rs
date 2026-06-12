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
        fn methods(
            values: Vec<confval::provenance::Located<String>>,
        ) -> Result<SmallVec<[Method; 4]>, ConfigError> {
            values
                .into_iter()
                .map(|s| {
                    Method::from_bytes(s.value.as_bytes()).map_err(|_| ConfigError::InvalidMethod {
                        value: s.value.clone(),
                        origin: "request_filter_device".to_string(),
                    })
                })
                .collect()
        }

        fn headers(
            values: Vec<confval::provenance::Located<String>>,
        ) -> Result<SmallVec<[HeaderName; 8]>, ConfigError> {
            values
                .into_iter()
                .map(|s| {
                    HeaderName::from_bytes(s.value.as_bytes()).map_err(|_| {
                        ConfigError::InvalidHeaderName {
                            value: s.value.clone(),
                            origin: "request_filter_device".to_string(),
                        }
                    })
                })
                .collect()
        }

        Ok(Self {
            enable: spec.enable.value,
            allow_methods: methods(spec.allow_methods)?,
            deny_methods: methods(spec.deny_methods)?,
            deny_headers: headers(spec.deny_headers)?,
            allow_headers: headers(spec.allow_headers)?,
            required_headers: headers(spec.required_headers)?,
            max_header_bytes: spec.max_header_bytes.value as usize,
            max_body_bytes: spec.max_body_bytes.value as usize,
            max_suspicious_body_bytes: spec.max_suspicious_body_bytes.value as usize,
            deny_status: spec.deny_status.map(|v| v.value as u16),
            client_body_timeout: spec
                .client_body_timeout_seconds
                .map(|v| Duration::from_secs(v.value as u64)),
            paths: spec.paths.into_iter().map(|p| p.value).collect(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::provenance::Located;

    fn default_spec() -> RequestFilterDeviceSpec {
        RequestFilterDeviceSpec {
            enable: Located::detached(true),
            ..Default::default()
        }
    }

    fn located_list(values: &[&str]) -> Vec<Located<String>> {
        values
            .iter()
            .map(|v| Located::detached(v.to_string()))
            .collect()
    }

    #[test]
    fn valid_methods_parsed() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            allow_methods: located_list(&["GET", "POST"]),
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
            allow_methods: located_list(&["INVALID METHOD"]),
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
            deny_headers: located_list(&["x-custom"]),
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
            deny_headers: located_list(&["invalid header!"]),
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
            client_body_timeout_seconds: Some(Located::detached(30)),
            ..default_spec()
        };

        // Act
        let config = RequestFilterDeviceConfig::try_from(spec).unwrap();

        // Assert
        assert_eq!(config.client_body_timeout, Some(Duration::from_secs(30)));
    }
}
