pub mod bind;
pub mod bind_admin_spec;
pub mod bind_interface_spec;
mod ingress_spec;

mod bind_admin_validation;
pub mod service;
pub mod static_files_spec;
mod static_files_validation;

pub use ingress_spec::*;
