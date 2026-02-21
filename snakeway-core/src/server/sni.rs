#[derive(Clone, Debug)]
pub struct DownstreamSni(pub String);

impl DownstreamSni {
    pub fn to_ascii_lowercase(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}
