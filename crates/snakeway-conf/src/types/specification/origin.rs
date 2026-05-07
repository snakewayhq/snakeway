use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[deprecated]
#[derive(Debug, Default, Clone, Serialize)]
pub struct OriginDeprecated {
    pub file: PathBuf,
    pub section: String,
    pub index: Option<usize>,
}

impl OriginDeprecated {
    pub fn new(file: &PathBuf, kind: &str, index: Option<usize>) -> Self {
        Self {
            file: file.into(),
            section: kind.to_owned(),
            index,
        }
    }

    pub fn test(message: &str) -> Self {
        Self::new(&PathBuf::from("/test/file"), message, None)
    }
}

impl fmt::Display for OriginDeprecated {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.index {
            Some(i) => write!(f, "{}: {}[{}] block", self.file.display(), self.section, i),
            None => write!(f, "{}: {} block", self.file.display(), self.section),
        }
    }
}
