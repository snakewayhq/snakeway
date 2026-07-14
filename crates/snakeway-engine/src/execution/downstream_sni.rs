#[derive(Clone, Debug)]
pub struct DownstreamSni(String);

impl DownstreamSni {
    pub fn new(host: String) -> Self {
        Self(host)
    }

    pub(crate) fn to_ascii_lowercase(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}
