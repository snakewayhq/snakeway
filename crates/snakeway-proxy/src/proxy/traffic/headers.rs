//! Write-back of device-visible headers onto Pingora header maps.
//!
//! Devices mutate headers on the request or response context, so the Pingora
//! message is rebuilt from that context after the pipeline runs. Clearing
//! first lets device Remove ops take effect. An insert-only loop would drop
//! removals and collapse appended multi-values.

use http::HeaderMap;
use http::header::HeaderName;
use pingora::http::{RequestHeader, ResponseHeader};

/// A Pingora header container that can be cleared and repopulated.
trait HeaderTarget {
    fn header_names(&self) -> Vec<HeaderName>;
    fn remove_header(&mut self, name: &HeaderName);
    fn append_header(
        &mut self,
        name: HeaderName,
        value: http::HeaderValue,
    ) -> pingora::Result<bool>;
}

impl HeaderTarget for RequestHeader {
    fn header_names(&self) -> Vec<HeaderName> {
        self.headers.keys().cloned().collect()
    }
    fn remove_header(&mut self, name: &HeaderName) {
        RequestHeader::remove_header(self, name);
    }
    fn append_header(
        &mut self,
        name: HeaderName,
        value: http::HeaderValue,
    ) -> pingora::Result<bool> {
        self.append_header(name, value)
    }
}

impl HeaderTarget for ResponseHeader {
    fn header_names(&self) -> Vec<HeaderName> {
        self.headers.keys().cloned().collect()
    }
    fn remove_header(&mut self, name: &HeaderName) {
        ResponseHeader::remove_header(self, name);
    }
    fn append_header(
        &mut self,
        name: HeaderName,
        value: http::HeaderValue,
    ) -> pingora::Result<bool> {
        self.append_header(name, value)
    }
}

/// Rebuild `target` from `headers`, i.e., remove every existing header,
/// then append the headers from `headers` in order. Shared by the request
/// and response write-back paths.
fn write_back<T: HeaderTarget>(target: &mut T, headers: &HeaderMap) -> pingora::Result<()> {
    for name in target.header_names() {
        target.remove_header(&name);
    }
    for (name, value) in headers {
        target.append_header(name.clone(), value.clone())?;
    }
    Ok(())
}

pub(in crate::proxy) fn write_back_request_headers(
    target: &mut RequestHeader,
    headers: &HeaderMap,
) -> pingora::Result<()> {
    write_back(target, headers)
}

pub(in crate::proxy) fn write_back_response_headers(
    target: &mut ResponseHeader,
    headers: &HeaderMap,
) -> pingora::Result<()> {
    write_back(target, headers)
}

#[cfg(test)]
mod tests {
    use super::*;
    use http::HeaderValue;
    use http::{Method, StatusCode};

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
