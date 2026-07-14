use crate::control_plane::bootstrap::start_control_plane;
use snakeway_conf::load_config;
use std::path::Path;
use std::process::exit;

pub fn start_server(config_path: &str, upgrade: bool, test: bool) {
    #[cfg(feature = "hotpath")]
    let _hotpath_guard = hotpath::HotpathGuardBuilder::new("snakeway")
        .percentiles(&[50.0, 95.0, 99.0])
        .format(hotpath::Format::Table)
        .build();

    let validated = match load_config(Path::new(&config_path)) {
        Ok(v) => v,
        Err(e) => {
            eprintln!("{e}");
            exit(1);
        }
    };

    if validated.has_warnings() {
        let mut out = String::new();
        validated.render_plain(&mut out);
        eprint!("{out}");
    }

    let config = validated.config;

    if test {
        println!("Configuration is valid.");
        return;
    }

    start_control_plane(config_path, config, upgrade).expect("Failed to start Snakeway server");
}
