use crate::conf::types::{IngressSpec, StaticFilesSpec, StaticRouteSpec};
use crate::conf::validation::{ValidationReport, validate_ingresses};
use std::path::PathBuf;

#[test]
fn validate_ingress_static_file_dir_does_not_exist() {
    // Arrange
    let file_dir = "/non/existent/static";
    let expected_error = format!("invalid static directory: {}", file_dir);
    let mut report = ValidationReport::default();
    let ingress = IngressSpec {
        static_cfgs: vec![StaticFilesSpec {
            routes: vec![StaticRouteSpec {
                file_dir: PathBuf::from(file_dir),
                ..Default::default()
            }],
            ..Default::default()
        }],
        ..Default::default()
    };

    // Act
    validate_ingresses(&[ingress], &mut report);

    // Assert
    assert_eq!(report.errors.first().unwrap().message, expected_error);
}
