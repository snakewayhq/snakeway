mod device;
mod entrypoint_spec;
pub(crate) mod field_emit;
mod ingress;
mod server;

pub use device::*;
pub use entrypoint_spec::*;
pub use ingress::*;
pub use server::*;

/// Signed integer type used in spec structs so that HCL deserialization
/// never fails on a numeric value. Validation and lowering narrow to the
/// correct target type.
pub type HclInt = i64;
