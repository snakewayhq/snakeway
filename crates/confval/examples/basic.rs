use confval::{
    RangeConstraint, SimpleOrigin, ValidateSpec, ValidationIssue, ValidationReport,
    range_constraint, validate_range_field,
};

range_constraint!(PORT, u16, min: 1, max: 65535);
range_constraint!(WORKERS, usize, min: 1, max: 512);

struct ServerConfig {
    port: u16,
    workers: usize,
    hostname: String,
}

impl ValidateSpec<SimpleOrigin> for ServerConfig {
    fn validate(&self, origin: &SimpleOrigin, report: &mut ValidationReport<SimpleOrigin>) {
        validate_range_field!(PORT, self.port, report, origin);
        validate_range_field!(WORKERS, self.workers, report, origin);

        if self.hostname.is_empty() {
            report.error(ValidationIssue::error_with_help(
                "hostname cannot be empty",
                origin.clone(),
                "Set hostname to a valid DNS name or IP address.",
            ));
        }
    }
}

fn main() {
    let config = ServerConfig {
        port: 0,
        workers: 2,
        hostname: String::new(),
    };
    let origin = SimpleOrigin::new("server.toml", "server block");
    let mut report = ValidationReport::default();

    config.validate(&origin, &mut report);

    if report.has_issues() {
        let mut out = String::new();
        report.render_plain(&mut out).unwrap();
        eprint!("{out}");
        std::process::exit(1);
    }
}
