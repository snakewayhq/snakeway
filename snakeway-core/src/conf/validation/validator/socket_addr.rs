/// Validates a socket address string by attempting to parse it into the target type.
///
/// # Parameters
/// * `value` - The string representation of the socket address to validate
/// * `report_fn` - A callback function to invoke if validation fails
///
/// # Examples
///
/// ```
/// # use snakeway_core::conf::validation::validator::socket_addr::validate_socket_addr;
/// # use snakeway_core::conf::validation::ValidationReport;
/// # let mut report = ValidationReport::default();
/// # let bind_admin = snakeway_core::conf::types::BindAdminSpec {
/// #     addr: "127.0.0.1:8080".to_string(),
/// #     origin: snakeway_core::conf::types::Origin::default(),
/// # };
/// let bind_admin_addr = validate_socket_addr(
///     &bind_admin.addr,
///     || report.invalid_bind_addr(&bind_admin.addr, &bind_admin.origin)
/// );
/// ```
pub fn validate_socket_addr<T, E, F>(value: &str, mut report_fn: F) -> Option<T>
where
    T: std::str::FromStr<Err = E>,
    F: FnMut(),
{
    value.parse().map_err(|_| report_fn()).ok()
}
