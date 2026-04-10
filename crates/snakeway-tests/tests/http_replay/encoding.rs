use super::replay_fixture;
use snakeway_tests::constants::HTTP_REPLAY_OK_RESPONSE;

/// A POST with a simple single-chunk body sent with Transfer-Encoding: chunked.
///
/// Chunked transfer encoding is the standard way to stream request bodies
/// in HTTP/1.1 when the total size is not known up-front.  The proxy must
/// correctly dechunk the body before forwarding to the upstream (or forward
/// the chunked stream transparently).  Either approach must result in the
/// upstream receiving a complete request.
#[test]
fn chunked_simple_body_should_proxy() {
    let resp = replay_fixture("encoding/chunked_simple.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "POST with simple chunked body should be proxied"
    );
}

/// A chunked body split across multiple chunks.
///
/// Verifies that the proxy correctly handles streaming framing where
/// a logical body (`foobar!`) is delivered as three separate chunks.
/// Some proxies only dechunk correctly when the body fits in a single
/// chunk; this test catches that regression.
#[test]
fn chunked_multi_chunk_body_should_proxy() {
    let resp = replay_fixture("encoding/chunked_multi_chunk.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "POST with multi-chunk body should be proxied"
    );
}

/// A chunked body where the chunk-size token contains non-hex characters.
///
/// RFC 9112 §7.1 defines chunk-size as `1*HEXDIG`.  A token like `ZZZZ`
/// is syntactically invalid. The proxy must not attempt to parse or forward
/// a request with an unparseable chunk size — doing so could result in
/// body mis-framing on the upstream side (a form of request smuggling).
#[test]
fn chunked_invalid_chunk_size_should_be_rejected() {
    let resp = replay_fixture("encoding/chunked_invalid_size.http");
    assert!(
        !resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "Chunked body with non-hex chunk size must not be proxied"
    );
}

/// A POST body consisting of 20 chunks of 10 bytes each (200 bytes total).
///
/// Tests that the proxy correctly handles a sustained multi-chunk stream
/// without prematurely closing the upstream connection or losing chunks.
/// This is the smallest scale at which multi-chunk reassembly bugs
/// typically manifest.
#[test]
fn large_chunked_body_should_proxy() {
    let resp = replay_fixture("encoding/large_body.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "POST with a large chunked body (multiple chunks) should be proxied"
    );
}

/// A POST with a Content-Encoding: gzip header and an opaque body.
///
/// Content-Encoding describes the encoding applied to the payload by the
/// client.  A reverse proxy MUST NOT decompress or re-encode the body —
/// it must pass it through opaquely.  Only the upstream application server
/// should decode the gzip content.
///
/// This test verifies that the proxy forwards Content-Encoding requests
/// without mangling them (even though the body here is not real gzip data;
/// the upstream mock ignores the body content and returns 200 regardless).
#[test]
fn content_encoding_gzip_passthrough_should_proxy() {
    let resp = replay_fixture("encoding/content_encoding_gzip_passthrough.http");
    assert!(
        resp.contains(HTTP_REPLAY_OK_RESPONSE),
        "POST with Content-Encoding: gzip should be proxied without modification"
    );
}
