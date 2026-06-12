use super::static_route_spec::{StaticRouteSpec, validate_static_route};
use confval::provenance::{Located, Report};
use serde::Serialize;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct StaticFilesSpec {
    #[confval(nested)]
    pub routes: Vec<Located<StaticRouteSpec>>,
}

pub fn validate_static_files(spec: &StaticFilesSpec, report: &mut Report) {
    for route in &spec.routes {
        validate_static_route(&route.value, report);
    }
}
