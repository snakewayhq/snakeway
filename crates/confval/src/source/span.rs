/// Identifies a source registered in a [`SourceMap`](crate::source::SourceMap).
///
/// Spans carry a `SourceId` so a single report can reference locations in
/// many files, and a single issue can relate spans across files.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct SourceId(pub(crate) u32);

impl SourceId {
    pub(crate) const DETACHED: SourceId = SourceId(u32::MAX);

    pub(crate) fn index(self) -> usize {
        self.0 as usize
    }
}

/// A byte range within one registered source.
///
/// Offsets are `u32`, capping a single source at 4 GiB. `Span` is `Copy`
/// and threaded everywhere by value.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub struct Span {
    pub source: SourceId,
    pub start: u32,
    pub end: u32,
}

impl Span {
    pub fn new(source: SourceId, start: u32, end: u32) -> Self {
        Self { source, start, end }
    }

    /// The smallest span covering both inputs.
    ///
    /// Both spans must belong to the same source; merging is intended for
    /// spans that are same-source by construction, such as two fields of the
    /// same block. A cross-source merge indicates a caller bug: it
    /// debug-asserts and returns `a`.
    pub fn merge(a: Span, b: Span) -> Span {
        debug_assert_eq!(
            a.source, b.source,
            "cannot merge spans from different sources"
        );
        if a.source != b.source {
            return a;
        }
        Span {
            source: a.source,
            start: a.start.min(b.start),
            end: a.end.max(b.end),
        }
    }

    /// Sentinel span for values with no source location, such as
    /// deserialized Specs or programmatically constructed test values.
    ///
    /// Rendering an issue with a detached span omits the location.
    pub fn detached() -> Span {
        Span {
            source: SourceId::DETACHED,
            start: 0,
            end: 0,
        }
    }

    pub fn is_detached(&self) -> bool {
        self.source == SourceId::DETACHED
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_produces_smallest_covering_span() {
        let source = SourceId(0);
        let a = Span::new(source, 5, 10);
        let b = Span::new(source, 20, 30);
        let merged = Span::merge(a, b);
        assert_eq!(merged, Span::new(source, 5, 30));
    }

    #[test]
    fn merge_is_order_independent() {
        let source = SourceId(0);
        let a = Span::new(source, 5, 10);
        let b = Span::new(source, 20, 30);
        assert_eq!(Span::merge(a, b), Span::merge(b, a));
    }

    #[test]
    fn merge_of_overlapping_spans() {
        let source = SourceId(0);
        let a = Span::new(source, 5, 25);
        let b = Span::new(source, 10, 30);
        assert_eq!(Span::merge(a, b), Span::new(source, 5, 30));
    }

    #[test]
    fn detached_span_is_detached() {
        assert!(Span::detached().is_detached());
        assert!(!Span::new(SourceId(0), 0, 1).is_detached());
    }
}
