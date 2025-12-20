#[derive(Debug)]
pub struct ByteRange {
    pub start: u64,
    pub end: u64, // inclusive
}

pub(crate) fn parse_range_header(header: &str, size: u64) -> Option<ByteRange> {
    let header = header.trim();

    if !header.starts_with("bytes=") {
        return None;
    }

    let range = &header[6..];
    let mut parts = range.split('-');

    let start = parts.next()?.parse::<u64>().ok()?;
    let end = match parts.next() {
        Some("") => size.saturating_sub(1),
        Some(v) => v.parse::<u64>().ok()?,
        None => return None,
    };

    if start > end || end >= size {
        return None;
    }

    Some(ByteRange { start, end })
}
