use crate::source::SourceId;

/// One unit of configuration input: a file, or a synthetic source such as an
/// environment variable.
///
/// Non-file sources are ordinary sources with a conventional name, e.g.
/// `sources.add("env:SNAKEWAY_PORT", "8080")`.
#[derive(Debug)]
pub struct Source {
    /// Display name, typically the path relative to the config root,
    /// e.g. "ingress.d/api.hcl".
    pub name: String,
    pub text: String,
    line_index: LineIndex,
}

impl Source {
    pub fn new(name: impl Into<String>, text: impl Into<String>) -> Self {
        let text = text.into();
        let line_index = LineIndex::new(&text);
        Self {
            name: name.into(),
            text,
            line_index,
        }
    }

    /// One-based line and column for a byte offset.
    ///
    /// The column counts characters, not bytes, so caret underlines align in
    /// the presence of multi-byte characters. Offsets past the end of the
    /// text are clamped.
    pub fn line_column(&self, byte_offset: u32) -> (usize, usize) {
        let offset = (byte_offset as usize).min(self.text.len());
        let line = self.line_index.line_at(offset as u32);
        let line_start = self.line_index.line_starts[line] as usize;
        let column = self.text[line_start..offset].chars().count() + 1;
        (line + 1, column)
    }

    /// The text of a one-based line, without its trailing newline.
    pub fn line_text(&self, line: usize) -> Option<&str> {
        let (start, end) = self.line_byte_range(line)?;
        Some(&self.text[start..end])
    }

    /// Byte range of a one-based line's content, excluding the trailing
    /// newline. Used by the renderer to clamp underlines to one line.
    pub(crate) fn line_byte_range(&self, line: usize) -> Option<(usize, usize)> {
        let starts = &self.line_index.line_starts;
        let start = *starts.get(line.checked_sub(1)?)? as usize;
        let end = starts
            .get(line)
            .map(|next| *next as usize)
            .unwrap_or(self.text.len());
        let content = self.text[start..end].trim_end_matches(['\n', '\r']);
        Some((start, start + content.len()))
    }
}

/// Interns sources and hands out the [`SourceId`]s that spans carry.
#[derive(Debug, Default)]
pub struct SourceMap {
    sources: Vec<Source>,
}

impl SourceMap {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add(&mut self, name: impl Into<String>, text: impl Into<String>) -> SourceId {
        assert!(
            self.sources.len() < SourceId::DETACHED.index(),
            "source map is full"
        );
        let id = SourceId(self.sources.len() as u32);
        self.sources.push(Source::new(name, text));
        id
    }

    /// Returns `None` for unknown ids and for the detached sentinel.
    pub fn get(&self, id: SourceId) -> Option<&Source> {
        self.sources.get(id.index())
    }
}

/// Precomputed line-start offsets for O(log n) offset-to-line conversion.
#[derive(Debug)]
struct LineIndex {
    line_starts: Vec<u32>,
}

impl LineIndex {
    fn new(text: &str) -> Self {
        let mut line_starts = vec![0u32];
        for (i, byte) in text.bytes().enumerate() {
            if byte == b'\n' {
                line_starts.push((i + 1) as u32);
            }
        }
        Self { line_starts }
    }

    /// Zero-based line index containing the byte offset.
    fn line_at(&self, offset: u32) -> usize {
        match self.line_starts.binary_search(&offset) {
            Ok(line) => line,
            Err(insertion) => insertion - 1,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn line_column_at_start_of_text() {
        let source = Source::new("test.hcl", "a = 1\nb = 2\n");
        assert_eq!(source.line_column(0), (1, 1));
    }

    #[test]
    fn line_column_within_first_line() {
        let source = Source::new("test.hcl", "a = 1\nb = 2\n");
        assert_eq!(source.line_column(4), (1, 5));
    }

    #[test]
    fn line_column_on_second_line() {
        let source = Source::new("test.hcl", "a = 1\nb = 2\n");
        assert_eq!(source.line_column(6), (2, 1));
        assert_eq!(source.line_column(10), (2, 5));
    }

    #[test]
    fn line_column_counts_characters_not_bytes() {
        // "é" is two bytes. The column after it must advance by one, not two.
        let source = Source::new("test.hcl", "x = \"é\"\ny = 1\n");
        let offset = source.text.find('y').unwrap() as u32;
        assert_eq!(source.line_column(offset), (2, 1));
        let quote_after = 4 + 1 + "é".len() as u32;
        assert_eq!(source.line_column(quote_after), (1, 7));
    }

    #[test]
    fn line_column_past_end_is_clamped() {
        let source = Source::new("test.hcl", "abc");
        assert_eq!(source.line_column(999), (1, 4));
    }

    #[test]
    fn line_text_returns_lines_without_newline() {
        let source = Source::new("test.hcl", "a = 1\nb = 2\n");
        assert_eq!(source.line_text(1), Some("a = 1"));
        assert_eq!(source.line_text(2), Some("b = 2"));
    }

    #[test]
    fn line_text_out_of_range_is_none() {
        let source = Source::new("test.hcl", "a = 1\n");
        assert_eq!(source.line_text(0), None);
        assert_eq!(source.line_text(99), None);
    }

    #[test]
    fn line_text_handles_missing_trailing_newline() {
        let source = Source::new("test.hcl", "a = 1\nb = 2");
        assert_eq!(source.line_text(2), Some("b = 2"));
    }

    #[test]
    fn source_map_add_and_get() {
        let mut sources = SourceMap::new();
        let first = sources.add("a.hcl", "x = 1");
        let second = sources.add("b.hcl", "y = 2");
        assert_eq!(sources.get(first).unwrap().name, "a.hcl");
        assert_eq!(sources.get(second).unwrap().name, "b.hcl");
    }

    #[test]
    fn source_map_detached_id_is_none() {
        let sources = SourceMap::new();
        assert!(sources.get(SourceId::DETACHED).is_none());
    }
}
