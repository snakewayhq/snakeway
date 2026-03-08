use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize, Clone)]
pub(crate) struct RedirectSpec {
    pub(crate) port: u16,
    pub(crate) status: u16,
}
