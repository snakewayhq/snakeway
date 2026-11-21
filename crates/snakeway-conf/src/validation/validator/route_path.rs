use confval::prelude::Format;

/// A route path prefix, which must start with `/`.
pub(crate) struct RoutePath;

impl Format for RoutePath {
    const NAME: &'static str = "route path";

    fn check(value: &str) -> bool {
        value.starts_with('/')
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_path_with_a_leading_slash_passes() {
        // Arrange
        let value = "/api";

        // Act
        let result = RoutePath::check(value);

        // Assert
        assert!(result);
    }

    #[test]
    fn a_path_without_a_leading_slash_fails() {
        // Arrange
        let value = "assets";

        // Act
        let result = RoutePath::check(value);

        // Assert
        assert!(!result);
    }
}
