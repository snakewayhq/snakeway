#[derive(Debug, PartialEq, Eq)]
pub(crate) struct ByteRange {
    pub(crate) start: u64,
    pub(crate) end: u64, // inclusive
}

impl ByteRange {
    /// Narrows the range to the bytes actually available, returning `None` when
    /// the start no longer exists, for example when a file shrank after the
    /// range was validated against the metadata size.
    pub(crate) fn fit_to_len(self, len: u64) -> Option<ByteRange> {
        if self.start >= len {
            return None;
        }
        Some(ByteRange {
            start: self.start,
            end: self.end.min(len - 1),
        })
    }
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_range_within_the_actual_length_is_kept() {
        // Arrange
        let range = ByteRange { start: 0, end: 3 };

        // Act
        let fitted = range.fit_to_len(5);

        // Assert
        assert_eq!(fitted, Some(ByteRange { start: 0, end: 3 }));
    }

    #[test]
    fn a_range_end_beyond_the_actual_length_is_clamped() {
        // Arrange
        let range = ByteRange {
            start: 0,
            end: 99_999,
        };

        // Act
        let fitted = range.fit_to_len(5);

        // Assert
        assert_eq!(fitted, Some(ByteRange { start: 0, end: 4 }));
    }

    #[test]
    fn a_range_starting_beyond_the_actual_length_is_dropped() {
        // Arrange
        let range = ByteRange { start: 10, end: 20 };

        // Act
        let fitted = range.fit_to_len(5);

        // Assert
        assert_eq!(fitted, None);
    }

    #[test]
    fn a_range_fit_to_an_empty_length_is_dropped() {
        // Arrange
        let range = ByteRange { start: 0, end: 9 };

        // Act
        let fitted = range.fit_to_len(0);

        // Assert
        assert_eq!(fitted, None);
    }
}
