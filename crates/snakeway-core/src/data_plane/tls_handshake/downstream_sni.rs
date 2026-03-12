#[derive(Clone, Debug)]
pub(crate) struct DownstreamSni(pub(crate) String);

impl DownstreamSni {
    pub(crate) fn to_ascii_lowercase(&self) -> String {
        self.0.to_ascii_lowercase()
    }
}
