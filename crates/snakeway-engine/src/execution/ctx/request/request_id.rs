use uuid::Uuid;

#[derive(Debug, Clone, Eq, Hash, PartialEq)]
pub struct RequestId(pub(crate) String);

impl RequestId {
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl Default for RequestId {
    fn default() -> Self {
        RequestId(Uuid::new_v4().to_string())
    }
}

impl From<String> for RequestId {
    fn from(s: String) -> Self {
        RequestId(s)
    }
}

impl From<&str> for RequestId {
    fn from(s: &str) -> Self {
        RequestId(s.to_owned())
    }
}
