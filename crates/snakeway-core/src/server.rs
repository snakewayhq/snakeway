use crate::control_plane::bootstrap::start_control_plane;
use snakeway_conf::load_config;
use std::path::Path;
use std::process::exit;

pub fn start_server(config_path: &str) {
    #[cfg(feature = "hotpath")]
    let _hotpath_guard = hotpath::HotpathGuardBuilder::new("snakeway")
        .percentiles(&[50.0, 95.0, 99.0])
        .format(hotpath::Format::Table)
        .build();

    let validated =
        load_config(Path::new(&config_path)).expect("Failed to load default Snakeway config");

    validated.validation_report.render_pretty();

    if validated.is_valid() {
        start_control_plane(config_path, validated.config)
            .expect("Failed to start Snakeway server");
    } else {
        exit(1);
    }
}
