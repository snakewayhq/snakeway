//! Write-back of device-visible headers onto Pingora header maps.
//!
//! Devices mutate headers on the request or response context, so the Pingora
//! message is rebuilt from that context after the pipeline runs. Clearing
//! first lets device Remove ops take effect. An insert-only loop would drop
//! removals and collapse appended multi-values.

use http::HeaderMap;
use http::header::HeaderName;
use pingora::http::{RequestHeader, ResponseHeader};

pub(in crate::proxy) fn write_back_request_headers(
    target: &mut RequestHeader,
    headers: &HeaderMap,
) -> pingora::Result<()> {
    let existing: Vec<HeaderName> = target.headers.keys().cloned().collect();
    for name in existing {
        target.remove_header(&name);
    }
    for (name, value) in headers {
        target.append_header(name.clone(), value)?;
    }
    Ok(())
}

pub(in crate::proxy) fn write_back_response_headers(
    target: &mut ResponseHeader,
    headers: &HeaderMap,
) -> pingora::Result<()> {
    let existing: Vec<HeaderName> = target.headers.keys().cloned().collect();
    for name in existing {
        target.remove_header(&name);
    }
    for (name, value) in headers {
        target.append_header(name.clone(), value)?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::{HeaderValue, Method, StatusCode};

    fn device_headers() -> HeaderMap {
        let mut headers = HeaderMap::new();
        headers.append("x-kept", HeaderValue::from_static("a"));
        headers.append("x-kept", HeaderValue::from_static("b"));
        headers
    }

    #[test]
    fn request_writeback_propagates_removals_and_multi_values() {
        // Arrange
        let mut target = RequestHeader::build(Method::GET, b"/", None).unwrap();
        target.append_header("x-removed-by-device", "gone").unwrap();
        let headers = device_headers();

        // Act
        write_back_request_headers(&mut target, &headers).unwrap();

        // Assert
        assert!(!target.headers.contains_key("x-removed-by-device"));
        let kept: Vec<_> = target.headers.get_all("x-kept").iter().collect();
        assert_eq!(kept, ["a", "b"]);
    }

    #[test]
    fn response_writeback_propagates_removals_and_multi_values() {
        // Arrange
        let mut target = ResponseHeader::build(StatusCode::OK, None).unwrap();
        target.append_header("x-removed-by-device", "gone").unwrap();
        let headers = device_headers();

        // Act
        write_back_response_headers(&mut target, &headers).unwrap();

        // Assert
        assert!(!target.headers.contains_key("x-removed-by-device"));
        let kept: Vec<_> = target.headers.get_all("x-kept").iter().collect();
        assert_eq!(kept, ["a", "b"]);
    }
}
