//! Echo upstreams: the proxy's forwarded bytes come back as JSON.
//!
//! The full echo upstream returns everything the proxy sent it, so tests
//! assert on forwarded bytes rather than on a canned response.

use super::plain::bind_ephemeral;
use std::io::{Read, Write};
use std::net::TcpStream;
use std::thread;
use std::time::Duration;

/// An upstream that reads the full request and echoes it back as a JSON
/// response body: the request line, every header as an ordered
/// `[name, value]` pair, the body as text, and the body byte count.
///
/// Response shape:
/// `{"request_line": "...", "headers": [["n","v"], ...], "body": "...", "body_bytes": N}`
pub fn start_http_upstream_that_echoes() -> u16 {
    let (listener, port) = bind_ephemeral();
    thread::spawn(move || {
        for stream in listener.incoming() {
            let mut stream = stream.expect("stream error");
            let _ = stream.set_read_timeout(Some(Duration::from_secs(30)));
            if let Some(request) = read_request(&mut stream) {
                let body = echo_json(&request);
                let resp = format!(
                    "HTTP/1.1 200 OK\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                let _ = stream.write_all(resp.as_bytes());
            }
        }
    });
    port
}

struct ParsedRequest {
    request_line: String,
    headers: Vec<(String, String)>,
    body: Vec<u8>,
}

/// Reads one request head plus a Content-Length or chunked body.
/// Returns None when the peer closes or errors before a head arrives.
fn read_request(stream: &mut TcpStream) -> Option<ParsedRequest> {
    let mut buf = Vec::new();
    let head_end = loop {
        if let Some(end) = find_head_end(&buf) {
            break end;
        }
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => return None,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => return None,
        }
    };

    let head = String::from_utf8_lossy(&buf[..head_end]).into_owned();
    let mut lines = head.split("\r\n");
    let request_line = lines.next().unwrap_or_default().to_string();
    let mut headers = Vec::new();
    for line in lines {
        match line.split_once(':') {
            Some((name, value)) => {
                headers.push((name.trim().to_lowercase(), value.trim().to_string()))
            }
            None => continue,
        }
    }

    let body = match header_value(&headers, "transfer-encoding") {
        Some(te) if te.to_ascii_lowercase().contains("chunked") => {
            read_chunked_body(stream, &mut buf, head_end)
        }
        _ => {
            let len = header_value(&headers, "content-length")
                .and_then(|v| v.trim().parse::<usize>().ok())
                .unwrap_or(0);
            read_sized_body(stream, &mut buf, head_end, len)
        }
    };

    Some(ParsedRequest {
        request_line,
        headers,
        body,
    })
}

fn find_head_end(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}

fn header_value(headers: &[(String, String)], name: &str) -> Option<String> {
    headers
        .iter()
        .find(|(n, _)| n == name)
        .map(|(_, v)| v.clone())
}

fn read_sized_body(
    stream: &mut TcpStream,
    buf: &mut Vec<u8>,
    head_end: usize,
    len: usize,
) -> Vec<u8> {
    while buf.len() < head_end + len {
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
    buf[head_end..(head_end + len).min(buf.len())].to_vec()
}

fn read_chunked_body(stream: &mut TcpStream, buf: &mut Vec<u8>, head_end: usize) -> Vec<u8> {
    let mut body = Vec::new();
    let mut pos = head_end;
    loop {
        fill_until(stream, buf, pos + 2);
        let Some(line_end) = find_crlf(&buf[pos..]) else {
            break;
        };
        let size_line = String::from_utf8_lossy(&buf[pos..pos + line_end]).into_owned();
        let size = usize::from_str_radix(size_line.split(';').next().unwrap_or("0").trim(), 16)
            .unwrap_or(0);
        pos += line_end + 2;
        if size == 0 {
            break;
        }
        fill_until(stream, buf, pos + size + 2);
        if buf.len() < pos + size {
            break;
        }
        body.extend_from_slice(&buf[pos..pos + size]);
        pos += size + 2;
    }
    body
}

fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

/// Reads until the buffer holds at least `needed` bytes, or the peer stops.
fn fill_until(stream: &mut TcpStream, buf: &mut Vec<u8>, needed: usize) {
    while buf.len() < needed {
        let mut tmp = [0u8; 4096];
        match stream.read(&mut tmp) {
            Ok(0) => break,
            Ok(n) => buf.extend_from_slice(&tmp[..n]),
            Err(_) => break,
        }
    }
}

/// Escapes a string for inclusion in the JSON response body.
fn json_escape(value: &str) -> String {
    let mut out = String::with_capacity(value.len());
    for c in value.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            '\r' => out.push_str("\\r"),
            '\t' => out.push_str("\\t"),
            c => out.push(c),
        }
    }
    out
}

fn echo_json(request: &ParsedRequest) -> String {
    let headers = request
        .headers
        .iter()
        .map(|(n, v)| format!("[\"{}\",\"{}\"]", json_escape(n), json_escape(v)))
        .collect::<Vec<_>>()
        .join(",");
    format!(
        "{{\"request_line\":\"{}\",\"headers\":[{}],\"body\":\"{}\",\"body_bytes\":{}}}",
        json_escape(&request.request_line),
        headers,
        json_escape(&String::from_utf8_lossy(&request.body)),
        request.body.len()
    )
}
