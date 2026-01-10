use serde::Serialize;
use std::fmt;
use std::path::PathBuf;

#[derive(Debug, Default, Clone, Serialize)]
pub struct Origin {
    pub(crate) file: PathBuf,
    kind: String,
    index: Option<usize>,
}

impl Origin {
    pub fn new(file: &PathBuf, kind: &str, index: Option<usize>) -> Self {
        Self {
            file: file.into(),
            kind: kind.to_owned(),
            index,
        }
    }
}

impl fmt::Display for Origin {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self.index {
            Some(i) => write!(f, "{}: {}[{}] block", self.file.display(), self.kind, i),
            None => write!(f, "{}: {} block", self.file.display(), self.kind),
        }
    }
}
