pub mod bind;
pub mod bind_admin;
pub mod bind_interface;
mod ingress_spec;

mod bind_admin_validation;
pub mod service;
pub mod static_files;
mod static_files_validation;

pub use ingress_spec::*;
