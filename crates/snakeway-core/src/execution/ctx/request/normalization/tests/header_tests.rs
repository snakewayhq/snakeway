use http::{HeaderName, HeaderValue};

// Parse and universal tests (Should be very few here).
#[test]
fn reject_nul_in_header_value_at_parse_time() {
    assert!(HeaderValue::from_bytes(b"abc\0def").is_err());
}

#[test]
fn header_name_auto_lowercased_at_parse_time() {
    let expected = "host";

    let result = HeaderName::from_bytes(b"Host").unwrap();

    assert_eq!(result.as_str(), expected);
}
