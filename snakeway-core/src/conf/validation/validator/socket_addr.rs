/// Checks if a string is a valid hostname according to DNS rules.
/// Validates length constraints (max 253 chars total, max 63 per label),
/// alphanumeric/hyphen characters only, and proper hyphen placement.
pub fn is_valid_hostname(s: &str) -> bool {
    if s.is_empty() || s.len() > 253 {
        return false;
    }

    s.split('.').all(|label| {
        !label.is_empty()
            && label.len() <= 63
            && label.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
            && !label.starts_with('-')
            && !label.ends_with('-')
    })
}

/// Checks if a port number is valid (must be greater than 0).
/// It is naturally bounded by the upper limit of u16 (65535).
pub const fn is_valid_port(port: u16) -> bool {
    port > 0
}
