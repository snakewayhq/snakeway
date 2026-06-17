use crate::source::Span;
use std::fmt;
use std::fmt::{Display, Formatter};
use std::hash::{Hash, Hasher};
use std::ops::Deref;

/// A parsed value together with its source span.
///
/// `Deref` makes the wrapper transparent at use sites: code reading
/// `Located<u16>` reads almost exactly like code reading `u16`. The span is
/// consulted only for error attribution.
///
/// # Equality ignores the span
///
/// Two `Located` values with equal inner values are equal, regardless of
/// where they came from. This is deliberate: a re-formatted but semantically
/// identical config must compare equal for diffing and rollback. `Hash`
/// agrees with `PartialEq`. If you need to compare locations, compare
/// `.span` explicitly.
#[derive(Clone, Debug)]
pub struct Located<T> {
    pub value: T,
    pub span: Span,
}

impl<T> Located<T> {
    pub fn new(value: T, span: Span) -> Self {
        Self { value, span }
    }

    /// A value with no source location (tests, programmatic construction).
    pub fn detached(value: T) -> Self {
        Self {
            value,
            span: Span::detached(),
        }
    }

    pub fn map<U>(self, f: impl FnOnce(T) -> U) -> Located<U> {
        Located {
            value: f(self.value),
            span: self.span,
        }
    }

    pub fn as_ref(&self) -> Located<&T> {
        Located {
            value: &self.value,
            span: self.span,
        }
    }
}

/// The default value is detached: it has no source location.
impl<T: Default> Default for Located<T> {
    fn default() -> Self {
        Located::detached(T::default())
    }
}

impl<T> Deref for Located<T> {
    type Target = T;
    fn deref(&self) -> &T {
        &self.value
    }
}

impl<T: Display> Display for Located<T> {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        Display::fmt(&self.value, f)
    }
}

impl<T: PartialEq> PartialEq for Located<T> {
    fn eq(&self, other: &Self) -> bool {
        self.value == other.value
    }
}

impl<T: Eq> Eq for Located<T> {}

impl<T: Hash> Hash for Located<T> {
    fn hash<H: Hasher>(&self, state: &mut H) {
        self.value.hash(state);
    }
}

/// Serializes the inner value transparently. Spans are meaningless across
/// runs, so persisted Specs round-trip values only.
#[cfg(feature = "serde")]
impl<T: serde::Serialize> serde::Serialize for Located<T> {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        self.value.serialize(serializer)
    }
}

/// Deserializes the inner value and attaches a detached span.
#[cfg(feature = "serde")]
impl<'de, T: serde::Deserialize<'de>> serde::Deserialize<'de> for Located<T> {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        T::deserialize(deserializer).map(Located::detached)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::SourceId;

    fn span_at(start: u32, end: u32) -> Span {
        Span::new(SourceId(0), start, end)
    }

    #[test]
    fn equality_ignores_span() {
        let a = Located::new(8080u16, span_at(0, 4));
        let b = Located::new(8080u16, span_at(100, 104));
        assert_eq!(a, b);
    }

    #[test]
    fn inequality_compares_values() {
        let a = Located::new(8080u16, span_at(0, 4));
        let b = Located::new(9090u16, span_at(0, 4));
        assert_ne!(a, b);
    }

    #[test]
    fn hash_agrees_with_equality() {
        use std::collections::HashSet;
        let mut set = HashSet::new();
        set.insert(Located::new("a".to_string(), span_at(0, 1)));
        set.insert(Located::new("a".to_string(), span_at(50, 51)));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn deref_reads_the_value() {
        let port = Located::new(8080u16, span_at(0, 4));
        assert_eq!(*port, 8080);
    }

    #[test]
    fn map_preserves_the_span() {
        let raw = Located::new("8080".to_string(), span_at(7, 11));
        let parsed = raw.map(|s| s.parse::<u16>().unwrap());
        assert_eq!(parsed.value, 8080);
        assert_eq!(parsed.span, span_at(7, 11));
    }

    #[test]
    fn as_ref_preserves_the_span() {
        let value = Located::new("x".to_string(), span_at(1, 2));
        let referenced = value.as_ref();
        assert_eq!(*referenced.value, "x");
        assert_eq!(referenced.span, span_at(1, 2));
    }

    #[test]
    fn display_delegates_to_the_value() {
        let value = Located::new(42, span_at(0, 2));
        assert_eq!(value.to_string(), "42");
    }

    #[test]
    fn detached_constructor_has_detached_span() {
        let value = Located::detached(1);
        assert!(value.span.is_detached());
    }

    #[cfg(feature = "serde")]
    #[test]
    fn serialize_is_transparent() {
        let value = Located::new(8080u16, span_at(0, 4));
        assert_eq!(serde_json::to_string(&value).unwrap(), "8080");
    }

    #[cfg(feature = "serde")]
    #[test]
    fn deserialize_produces_detached_span() {
        let value: Located<u16> = serde_json::from_str("8080").unwrap();
        assert_eq!(value.value, 8080);
        assert!(value.span.is_detached());
    }
}
