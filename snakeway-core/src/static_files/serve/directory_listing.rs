use std::path::PathBuf;

use crate::static_files::StaticResponse;
use bytes::Bytes;
use http::{HeaderMap, HeaderValue, StatusCode};

use crate::static_files::StaticBody;
use percent_encoding::{AsciiSet, CONTROLS, utf8_percent_encode};
use std::fs::DirEntry;

/// Render a basic HTML directory listing.
/// Assumes:
/// - `dir` is already canonicalized and validated
/// - traversal has already been prevented
/// - caller has confirmed directory_listing is enabled
pub fn serve_directory_listing(dir: PathBuf, request_path: &str) -> StaticResponse {
    let mut entries = match std::fs::read_dir(&dir) {
        Ok(rd) => rd
            .filter_map(|e| e.ok())
            .filter(|e| !is_hidden(e))
            .collect::<Vec<_>>(),
        Err(_) => {
            return StaticResponse {
                status: StatusCode::FORBIDDEN,
                headers: HeaderMap::new(),
                body: StaticBody::Empty,
            };
        }
    };

    // Sort: directories first, then files, lexicographically
    entries.sort_by(|a, b| {
        let a_is_dir = a.file_type().map(|t| t.is_dir()).unwrap_or(false);
        let b_is_dir = b.file_type().map(|t| t.is_dir()).unwrap_or(false);

        match (a_is_dir, b_is_dir) {
            (true, false) => std::cmp::Ordering::Less,
            (false, true) => std::cmp::Ordering::Greater,
            _ => a.file_name().cmp(&b.file_name()),
        }
    });

    let mut html = String::with_capacity(4096);

    html.push_str("<!DOCTYPE html>\n");
    html.push_str("<html>\n<head>\n");
    html.push_str("<meta charset=\"utf-8\">\n");

    html.push_str("<title>Index of ");
    html.push_str(&escape_html(request_path));
    html.push_str("</title>\n");

    html.push_str("</head>\n<body>\n");

    html.push_str("<h1>Index of ");
    html.push_str(&escape_html(request_path));
    html.push_str("</h1>\n");

    html.push_str("<ul>\n");

    // Parent link (unless at root)
    if request_path != "/" {
        html.push_str("<li><a href=\"../\">../</a></li>\n");
    }

    for entry in entries {
        let name = entry.file_name();
        let name = name.to_string_lossy();

        let is_dir = entry.file_type().map(|t| t.is_dir()).unwrap_or(false);

        html.push_str("<li><a href=\"");
        html.push_str(&escape_href(&name));
        if is_dir {
            html.push('/');
        }
        html.push_str("\">");
        html.push_str(&escape_html(&name));
        if is_dir {
            html.push('/');
        }
        html.push_str("</a></li>\n");
    }

    html.push_str("</ul>\n");
    html.push_str("</body>\n</html>\n");

    let body: Bytes = html.into();

    let mut headers = HeaderMap::new();
    headers.insert(
        http::header::CONTENT_TYPE,
        HeaderValue::from_static("text/html; charset=utf-8"),
    );
    headers.insert(
        http::header::CACHE_CONTROL,
        HeaderValue::from_static("no-store"),
    );
    headers.insert(
        http::header::CONTENT_LENGTH,
        HeaderValue::from_str(&body.len().to_string()).unwrap(),
    );

    StaticResponse {
        status: StatusCode::OK,
        headers,
        body: StaticBody::Bytes(body),
    }
}

/// Hide dotfiles by default
fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(true)
}

/// Minimal HTML escaping (sufficient for filenames)
fn escape_html(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for c in input.chars() {
        match c {
            '&' => out.push_str("&amp;"),
            '<' => out.push_str("&lt;"),
            '>' => out.push_str("&gt;"),
            '"' => out.push_str("&quot;"),
            '\'' => out.push_str("&#x27;"),
            _ => out.push(c),
        }
    }
    out
}

/// Encode a path segment for use in an HTML href attribute.
/// This is URL encoding, NOT HTML escaping.
fn escape_href(input: &str) -> String {
    // RFC 3986 unreserved characters: ALPHA / DIGIT / "-" / "." / "_" / "~"
    // Everything else gets percent-encoded.
    const FRAGMENT: &AsciiSet = &CONTROLS
        .add(b' ')
        .add(b'"')
        .add(b'<')
        .add(b'>')
        .add(b'`')
        .add(b'#')
        .add(b'?')
        .add(b'%');

    utf8_percent_encode(input, FRAGMENT).to_string()
}
