use crate::conf::types::{BindAdminSpec, BindSpec, Origin, ServiceSpec, StaticFilesSpec};
use serde::{Deserialize, Serialize};

/// The operator DSL for the config subsystem.
/// This defines the configuration file format of files in ./config/ingress.d/*.hcl
#[derive(Debug, Deserialize, Serialize, Default)]
#[serde(rename_all = "snake_case")]
pub(crate) struct IngressSpec {
    #[serde(skip)]
    pub(crate) origin: Origin,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bind: Option<BindSpec>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) bind_admin: Option<BindAdminSpec>,
    pub(crate) services: Vec<ServiceSpec>,
    pub(crate) static_files: Vec<StaticFilesSpec>,
}
