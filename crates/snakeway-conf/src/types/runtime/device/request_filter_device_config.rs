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
