//! Guards the span fidelity the `confval::hcl` adapter depends on: hcl-edit
//! must expose byte-accurate spans for attribute keys and values, nested
//! block attributes, individual array elements, and whole blocks. If an
//! hcl-edit upgrade breaks any of these, error attribution breaks with it.

#![cfg(feature = "hcl")]

use hcl_edit::Span as _;
use hcl_edit::parser::parse_body;

const INPUT: &str = r#"server {
  hostname = "example.com"
  port = 8080
  daemon = true

  tls {
    cert = "cert.pem"
  }

  allow = ["10.0.0.0/8", "not a cidr"]
}
"#;

#[test]
fn attribute_value_spans_are_byte_accurate() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();

    let hostname = server.body.get_attribute("hostname").unwrap();
    assert_eq!(&INPUT[hostname.value.span().unwrap()], "\"example.com\"");

    let port = server.body.get_attribute("port").unwrap();
    assert_eq!(&INPUT[port.value.span().unwrap()], "8080");

    let daemon = server.body.get_attribute("daemon").unwrap();
    assert_eq!(&INPUT[daemon.value.span().unwrap()], "true");
}

#[test]
fn attribute_key_spans_are_byte_accurate() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();

    let hostname = server.body.get_attribute("hostname").unwrap();
    assert_eq!(&INPUT[hostname.key.span().unwrap()], "hostname");
}

#[test]
fn nested_block_attribute_spans_are_byte_accurate() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();
    let tls = server.body.get_blocks("tls").next().unwrap();

    let cert = tls.body.get_attribute("cert").unwrap();
    assert_eq!(&INPUT[cert.value.span().unwrap()], "\"cert.pem\"");
}

#[test]
fn array_elements_have_individual_spans() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();

    let allow = server.body.get_attribute("allow").unwrap();
    let array = allow.value.as_array().unwrap();
    let texts: Vec<&str> = array
        .iter()
        .map(|element| &INPUT[element.span().unwrap()])
        .collect();
    assert_eq!(texts, vec!["\"10.0.0.0/8\"", "\"not a cidr\""]);
}

#[test]
fn block_span_covers_header_through_closing_brace() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();

    let text = &INPUT[server.span().unwrap()];
    assert!(text.starts_with("server {"), "got: {text:?}");
    assert!(text.ends_with('}'), "got: {text:?}");
}

#[test]
fn nested_block_span_covers_header_through_closing_brace() {
    let body = parse_body(INPUT).unwrap();
    let server = body.blocks().next().unwrap();
    let tls = server.body.get_blocks("tls").next().unwrap();

    let text = &INPUT[tls.span().unwrap()];
    assert!(text.starts_with("tls {"), "got: {text:?}");
    assert!(text.ends_with('}'), "got: {text:?}");
}
