use crate::conf::types::IdentityDeviceSpec;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct IdentityDeviceSpecRoot {
    pub identity_device: IdentityDeviceSpec,
}

impl From<IdentityDeviceSpec> for IdentityDeviceSpecRoot {
    fn from(identity_device: IdentityDeviceSpec) -> Self {
        Self { identity_device }
    }
}
