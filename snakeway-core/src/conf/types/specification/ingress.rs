use crate::conf::types::{BindAdminSpec, BindSpec, Origin, ServiceSpec, StaticFilesSpec};
use serde::{Deserialize, Serialize};

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub struct IngressSpec {
    #[serde(skip)]
    pub origin: Origin,
    pub bind: Option<BindSpec>,
    pub bind_admin: Option<BindAdminSpec>,
    pub services: Vec<ServiceSpec>,
    pub static_files: Vec<StaticFilesSpec>,
}
