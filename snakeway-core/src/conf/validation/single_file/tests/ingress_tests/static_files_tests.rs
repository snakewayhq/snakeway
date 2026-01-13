use crate::conf::types::{IngressSpec, StaticFilesSpec, StaticRouteSpec};
use crate::conf::validation::{ValidationReport, validate_ingresses};
use std::path::PathBuf;

fn minimal_static_files_ingress(file_dir: &str) -> IngressSpec {
    IngressSpec {
        static_cfgs: vec![StaticFilesSpec {
            routes: vec![StaticRouteSpec {
                file_dir: PathBuf::from(file_dir),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    }
}

#[test]
fn validate_ingress_static_file_dir_does_not_exist() {
    // Arrange
    let file_dir = "/non/existent/static";
    let expected_error = format!("invalid static directory: {}", file_dir);
    let mut report = ValidationReport::default();
    let ingress = minimal_static_files_ingress(file_dir);

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors.first().unwrap().message, expected_error);
}

#[test]
fn validate_static_file_dir_is_not_relative() {
    // Arrange
    let file_dir = "./public";
    let expected_error0 = format!("invalid static directory: {}", file_dir);
    let expected_error1 = format!(
        "static file directory must be an absolute path: {}",
        file_dir
    );
    let mut report = ValidationReport::default();
    let ingress = minimal_static_files_ingress(file_dir);

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors[0].message, expected_error0);
    assert_eq!(report.errors[1].message, expected_error1);
}
