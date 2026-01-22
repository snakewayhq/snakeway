use crate::conf::types::RequestFilterDeviceSpec;
use crate::conf::validation::ConfigError;
use http::{HeaderName, Method};
use serde::{Deserialize, Serialize};

#[derive(Default, Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RequestFilterDeviceConfig {
    pub enable: bool,
    #[serde(with = "serde_method_vec")]
    pub allow_methods: Vec<Method>,
    #[serde(with = "serde_method_vec")]
    pub deny_methods: Vec<Method>,
    #[serde(with = "serde_header_name_vec")]
    pub deny_headers: Vec<HeaderName>,
    #[serde(with = "serde_header_name_vec")]
    pub allow_headers: Vec<HeaderName>,
    #[serde(with = "serde_header_name_vec")]
    pub required_headers: Vec<HeaderName>,
    pub max_header_bytes: usize,
    pub max_body_bytes: usize,
    pub deny_status: Option<u16>,
}

impl TryFrom<RequestFilterDeviceSpec> for RequestFilterDeviceConfig {
    type Error = ConfigError;

    fn try_from(spec: RequestFilterDeviceSpec) -> Result<Self, Self::Error> {
        let origin = spec.origin.clone();

        let allow_methods = spec
            .allow_methods
            .into_iter()
            .map(|s| {
                Method::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidMethod {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let deny_methods = spec
            .deny_methods
            .into_iter()
            .map(|s| {
                Method::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidMethod {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let deny_headers = spec
            .deny_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let allow_headers = spec
            .allow_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        let required_headers = spec
            .required_headers
            .into_iter()
            .map(|s| {
                HeaderName::from_bytes(s.as_bytes()).map_err(|_| ConfigError::InvalidHeaderName {
                    value: s,
                    origin: origin.to_string(),
                })
            })
            .collect::<Result<Vec<_>, _>>()?;

        Ok(Self {
            enable: spec.enable,
            allow_methods,
            deny_methods,
            deny_headers,
            allow_headers,
            required_headers,
            max_header_bytes: spec.max_header_bytes,
            max_body_bytes: spec.max_body_bytes,
            deny_status: spec.deny_status,
        })
    }
}

// Serialization helpers
mod serde_header_name_vec {
    use http::HeaderName;
    use serde::{Deserialize, Deserializer, Serialize, Serializer};

    pub fn serialize<S>(headers: &Vec<HeaderName>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&str> = headers.iter().map(|h| h.as_str()).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<HeaderName>, D::Error>
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

    pub fn serialize<S>(methods: &Vec<Method>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let strings: Vec<&str> = methods.iter().map(|m| m.as_str()).collect();
        strings.serialize(serializer)
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<Method>, D::Error>
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
