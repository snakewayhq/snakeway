use confval::prelude::{Located, Report};

pub(crate) fn validate_device_paths(paths: &[Located<String>], report: &mut Report) {
    for path in paths {
        if !path.value.starts_with('/') {
            report
                .error(format!("device path must start with '/': {}", path.value))
                .at(path.span)
                .emit();
        }
    }
}
