//! Response head and chunked body parsing over a received byte buffer.
//!
//! Parsing is pure: it reads a buffer and reports where a boundary sits, or
//! reports that more bytes are needed. The replay engine owns the reads.

/// The framing rule a response body follows.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BodyKind {
    /// No body: HEAD, 204, 304, or an interim response.
    None,
    /// Exactly Content-Length bytes.
    Sized(u64),
    /// Chunked framing through the terminal chunk and trailers.
    Chunked,
    /// No framing rule: read until the peer closes.
    UntilClose,
}

/// One parsed response head.
#[derive(Debug, Clone)]
pub struct ResponseHead {
    /// The response status code.
    pub status: u16,
    /// Absolute buffer offset just past the head terminator.
    pub head_len: usize,
    header_names: Vec<String>,
    header_values: Vec<String>,
}

impl ResponseHead {
    /// Interim responses (1xx other than 101) never carry a body and are
    /// followed by a final response on the same connection.
    pub fn is_interim(&self) -> bool {
        self.status >= 100 && self.status < 200 && self.status != 101
    }

    /// The framing rule this response's body follows.
    pub fn body_kind(&self, is_head: bool) -> BodyKind {
        if is_head || self.status == 204 || self.status == 304 {
            return BodyKind::None;
        }
        if self.is_interim() {
            return BodyKind::None;
        }
        if self.status == 101 {
            return BodyKind::UntilClose;
        }
        if let Some(te) = self.header("transfer-encoding")
            && te.to_ascii_lowercase().contains("chunked")
        {
            return BodyKind::Chunked;
        }
        if let Some(cl) = self.header("content-length")
            && let Ok(len) = cl.trim().parse::<u64>()
        {
            return BodyKind::Sized(len);
        }
        BodyKind::UntilClose
    }

    /// The first value of a header, matched case insensitively.
    pub fn header(&self, name: &str) -> Option<&str> {
        self.header_names
            .iter()
            .position(|n| n.eq_ignore_ascii_case(name))
            .map(|index| self.header_values[index].as_str())
    }
}

/// Parses one response head starting at `from`. Returns None while the head
/// is incomplete or does not start with an HTTP version line.
pub fn parse_response_head(buf: &[u8], from: usize) -> Option<ResponseHead> {
    let window = buf.get(from..)?;
    let end = window.windows(4).position(|w| w == b"\r\n\r\n")? + from;
    let head_text = std::str::from_utf8(&buf[from..end + 4]).ok()?;

    let mut lines = head_text.split("\r\n");
    let status_line = lines.next()?;
    if !status_line.starts_with("HTTP/") {
        return None;
    }
    let status = status_line.split(' ').nth(1)?.parse::<u16>().ok()?;

    let mut header_names = Vec::new();
    let mut header_values = Vec::new();
    for line in lines {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        header_names.push(name.trim().to_lowercase());
        header_values.push(value.trim().to_string());
    }

    Some(ResponseHead {
        status,
        head_len: end + 4,
        header_names,
        header_values,
    })
}

/// Parses a chunked body starting at `from` and returns the absolute offset
/// just past the terminal chunk's trailing CRLF and its trailers. Returns
/// None while the framing is incomplete, and on a chunk size the buffer
/// cannot reach, which is how an oversized size line reports instead of
/// overflowing the arithmetic.
pub fn parse_chunked(buf: &[u8], from: usize) -> Option<usize> {
    let mut cursor = from;
    loop {
        let line_end = find_crlf(buf, cursor)?;
        let size_line = std::str::from_utf8(&buf[cursor..line_end]).ok()?;
        let size = usize::from_str_radix(size_line.split(';').next()?.trim(), 16).ok()?;
        cursor = line_end.checked_add(2)?;
        if size == 0 {
            return parse_trailers(buf, cursor);
        }
        let data_end = cursor.checked_add(size)?.checked_add(2)?;
        if buf.len() < data_end {
            return None;
        }
        cursor = data_end;
    }
}

/// Parses trailer fields after the terminal chunk: CRLF terminated lines up
/// to an empty line. Returns the offset past the empty line.
fn parse_trailers(buf: &[u8], mut cursor: usize) -> Option<usize> {
    loop {
        let line_end = find_crlf(buf, cursor)?;
        if line_end == cursor {
            return Some(cursor + 2);
        }
        cursor = line_end + 2;
    }
}

fn find_crlf(buf: &[u8], from: usize) -> Option<usize> {
    buf.get(from..)?
        .windows(2)
        .position(|w| w == b"\r\n")
        .map(|p| p + from)
}
