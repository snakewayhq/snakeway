use std::fmt::{Display, Formatter};

/// Represents an error that occurred during device-related operations.
///
/// This error type encapsulates a string message describing what went wrong
/// during device operations in the Snakeway proxy.
#[derive(Debug)]
pub struct DeviceError {
    /// A descriptive message explaining the error that occurred
    pub message: String,
    /// Whether the error is considered fatal and should be reported to the client
    pub fatal: bool,
}

impl Display for DeviceError {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        let fatal = if self.fatal { "(fatal) " } else { "" };
        write!(f, "{}{}", fatal, self.message)
    }
}
