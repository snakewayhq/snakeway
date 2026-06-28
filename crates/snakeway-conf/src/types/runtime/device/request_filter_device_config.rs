use crate::types::RequestFilterDeviceSpec;
use confval::prelude::{Located, Lower, Report, Validate, narrow};
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

impl Lower<RequestFilterDeviceSpec> for RequestFilterDeviceConfig
where
    RequestFilterDeviceSpec: Validate,
{
    fn lower(spec: &RequestFilterDeviceSpec, report: &mut Report) -> Option<Self> {
        fn methods(
            values: &[Located<String>],
            report: &mut Report,
            ok: &mut bool,
        ) -> SmallVec<[Method; 4]> {
            values
                .iter()
                .filter_map(|s| match Method::from_bytes(s.value.as_bytes()) {
                    Ok(method) => Some(method),
                    Err(_) => {
                        report
                            .error(format!("invalid method: {}", s.value))
                            .at(s.span)
                            .emit();
                        *ok = false;
                        None
                    }
                })
                .collect()
        }

        fn headers(
            values: &[Located<String>],
            report: &mut Report,
            ok: &mut bool,
        ) -> SmallVec<[HeaderName; 8]> {
            values
                .iter()
                .filter_map(|s| match HeaderName::from_bytes(s.value.as_bytes()) {
                    Ok(header) => Some(header),
                    Err(_) => {
                        report
                            .error(format!("invalid header name: {}", s.value))
                            .at(s.span)
                            .emit();
                        *ok = false;
                        None
                    }
                })
                .collect()
        }

        // Size and timeout limits narrow through `narrow::` rather than `as`,
        // so a negative value is reported and rejected instead of wrapping to
        // a near-unbounded `usize`/`u64` and silently disabling the limit.
        fn bytes(located: &Located<i64>, report: &mut Report, ok: &mut bool) -> usize {
            narrow::i64_to_usize(located, report).unwrap_or_else(|| {
                *ok = false;
                0
            })
        }

        let mut ok = true;

        let client_body_timeout =
            match narrow::opt_i64_secs_to_duration(&spec.client_body_timeout_seconds, report) {
                Some(timeout) => timeout,
                None => {
                    ok = false;
                    None
                }
            };

        let config = Self {
            enable: spec.enable.value,
            allow_methods: methods(&spec.allow_methods, report, &mut ok),
            deny_methods: methods(&spec.deny_methods, report, &mut ok),
            deny_headers: headers(&spec.deny_headers, report, &mut ok),
            allow_headers: headers(&spec.allow_headers, report, &mut ok),
            required_headers: headers(&spec.required_headers, report, &mut ok),
            max_header_bytes: bytes(&spec.max_header_bytes, report, &mut ok),
            max_body_bytes: bytes(&spec.max_body_bytes, report, &mut ok),
            max_suspicious_body_bytes: bytes(&spec.max_suspicious_body_bytes, report, &mut ok),
            deny_status: spec.deny_status.as_ref().map(|v| v.value as u16),
            client_body_timeout,
            paths: spec.paths.iter().map(|p| p.value.clone()).collect(),
        };
        ok.then_some(config)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use confval::prelude::Located;

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
    fn negative_byte_limit_is_rejected_not_wrapped() {
        // Arrange: a negative max_body_bytes must not lower to usize::MAX and
        // silently remove the cap.
        let spec = RequestFilterDeviceSpec {
            max_body_bytes: Located::detached(-1),
            ..default_spec()
        };
        let mut report = Report::new();

        // Act
        let result = RequestFilterDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.has_errors());
    }

    #[test]
    fn negative_client_body_timeout_is_rejected() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            client_body_timeout_seconds: Some(Located::detached(-1)),
            ..default_spec()
        };
        let mut report = Report::new();

        // Act
        let result = RequestFilterDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(report.has_errors());
    }

    #[test]
    fn valid_methods_parsed() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            allow_methods: located_list(&["GET", "POST"]),
            ..default_spec()
        };
        let mut report = Report::new();

        // Act
        let config = RequestFilterDeviceConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert!(!report.has_errors());
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
        let mut report = Report::new();

        // Act
        let result = RequestFilterDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "invalid method: INVALID METHOD")
        );
    }

    #[test]
    fn valid_headers_parsed() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            deny_headers: located_list(&["x-custom"]),
            ..default_spec()
        };
        let mut report = Report::new();

        // Act
        let config = RequestFilterDeviceConfig::lower(&spec, &mut report).unwrap();

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
        let mut report = Report::new();

        // Act
        let result = RequestFilterDeviceConfig::lower(&spec, &mut report);

        // Assert
        assert!(result.is_none());
        assert!(
            report
                .issues()
                .iter()
                .any(|i| i.message == "invalid header name: invalid header!")
        );
    }

    #[test]
    fn client_body_timeout_converted() {
        // Arrange
        let spec = RequestFilterDeviceSpec {
            client_body_timeout_seconds: Some(Located::detached(30)),
            ..default_spec()
        };
        let mut report = Report::new();

        // Act
        let config = RequestFilterDeviceConfig::lower(&spec, &mut report).unwrap();

        // Assert
        assert_eq!(config.client_body_timeout, Some(Duration::from_secs(30)));
    }
}
