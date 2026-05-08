use std::fmt::{Debug, Display, Formatter, Result};

pub trait Origin: Display + Debug + Clone {
    /// A grouping key for rendering (file path, source name, etc.)
    /// Issues with the same `source()` are grouped together in reports.
    fn source(&self) -> &str;
}

#[derive(Debug, Clone, Default)]
pub struct SimpleOrigin {
    pub source: String,
    pub context: String,
}

impl SimpleOrigin {
    pub fn new(source: impl Into<String>, context: impl Into<String>) -> Self {
        Self {
            source: source.into(),
            context: context.into(),
        }
    }
}

impl Display for SimpleOrigin {
    fn fmt(&self, f: &mut Formatter<'_>) -> Result {
        write!(f, "{}: {}", self.source, self.context)
    }
}

impl Origin for SimpleOrigin {
    fn source(&self) -> &str {
        &self.source
    }
}
