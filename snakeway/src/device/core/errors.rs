/// Represents an error that occurred during device-related operations.
///
/// This error type encapsulates a string message describing what went wrong
/// during device operations in the Snakeway proxy.
#[derive(Debug)]
pub struct DeviceError {
    /// A descriptive message explaining the error that occurred
    pub message: String,
}

impl DeviceError {
    /// Creates a new `DeviceError` with the given error message.
    ///
    /// # Arguments
    ///
    /// * `msg` - Any type that can be converted into a String that describes the error
    ///
    /// # Returns
    ///
    /// Returns a new `DeviceError` instance containing the provided message
    pub fn new(msg: impl Into<String>) -> Self {
        Self {
            message: msg.into(),
        }
    }
}
