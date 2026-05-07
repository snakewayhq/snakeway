use crate::issue::ValidationIssue;
use crate::origin::Origin;
use std::fmt;

#[derive(Debug, Default)]
pub struct ValidationReport<O: Origin> {
    errors: Vec<ValidationIssue<O>>,
    warnings: Vec<ValidationIssue<O>>,
}

impl<O: Origin> ValidationReport<O> {
    /// Collect error
    pub fn error(&mut self, issue: ValidationIssue<O>) {
        self.errors.push(issue);
    }

    /// Collect warning
    pub fn warning(&mut self, issue: ValidationIssue<O>) {
        self.warnings.push(issue);
    }

    /// Collect and auto-route by severity.
    pub fn push(&mut self, issue: ValidationIssue<O>) {
        match issue.severity {
            crate::severity::Severity::Error => self.error(issue),
            crate::severity::Severity::Warning => self.warning(issue),
        }
    }

    pub fn has_errors(&self) -> bool {
        !self.errors.is_empty()
    }
    pub fn has_warnings(&self) -> bool {
        !self.warnings.is_empty()
    }

    pub fn has_issues(&self) -> bool {
        self.has_errors() || self.has_warnings()
    }

    pub fn errors(&self) -> &[ValidationIssue<O>] {
        &self.errors
    }

    pub fn warnings(&self) -> &[ValidationIssue<O>] {
        &self.warnings
    }

    /// Iterate over errors then warnings.
    pub fn iter(&self) -> impl Iterator<Item = &ValidationIssue<O>> {
        self.errors.iter().chain(self.warnings.iter())
    }

    /// Merge two reports into one.
    pub fn merge(&mut self, other: ValidationReport<O>) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
    }
}

impl<O: Origin> ValidationReport<O> {
    /// Compact, one-line-per-issue format for CI/scripts.
    pub fn render_plain(&self, w: &mut impl fmt::Write) -> fmt::Result {
        for issue in self.iter() {
            writeln!(w, "{}: {}", issue.origin.source(), issue.message)?;
        }
        Ok(())
    }

    /// Colorized, grouped-by-source format for terminals.
    /// Requires the `color` feature (pulls in `owo-colors`).
    #[cfg(feature = "color")]
    pub fn render_pretty(&self, w: &mut impl fmt::Write) -> fmt::Result {
        todo!("implement render_pretty")
    }
}

// Behind `serde` feature
#[cfg(feature = "serde")]
impl<O: Origin + Serialize> ValidationReport<O> {
    pub fn render_json(&self, w: &mut impl io::Write) -> serde_json::Result<()> {
        todo!("implement render_json")
    }
}
