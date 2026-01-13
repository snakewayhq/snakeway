use crate::conf::types::{Origin, TlsSpec};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct BindAdminSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub addr: String,
    pub tls: TlsSpec,
}
