use http::{HeaderMap, HeaderName, HeaderValue};

pub(crate) fn input_to_header_map(input: &[(&str, &str)]) -> HeaderMap {
    let mut header_map = HeaderMap::new();
    for (k, v) in input {
        let name: HeaderName = k.parse().expect("invalid header name");
        let value: HeaderValue = v.parse().expect("invalid header value");
        header_map.append(name, value);
    }
    header_map
}
