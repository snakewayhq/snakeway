use crate::conf::load_config;
use crate::control_plane::bootstrap::start_control_plane;
use std::path::Path;
use std::process::exit;

pub fn start_server(config_path: &str) {
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
