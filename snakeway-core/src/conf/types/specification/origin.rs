use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Origin {
    pub(crate) file: PathBuf,
    pub(crate) section: String,
    pub(crate) index: Option<usize>,
}

impl Origin {
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

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.index {
            Some(i) => write!(f, "{}: {}[{}] block", self.file.display(), self.section, i),
            None => write!(f, "{}: {} block", self.file.display(), self.section),
        }
    }
}
