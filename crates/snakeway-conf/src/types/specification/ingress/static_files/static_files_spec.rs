use super::static_route_spec::{StaticRouteSpec, validate_static_route};
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct StaticFilesSpec {
    #[confval(nested)]
    pub routes: Vec<Located<StaticRouteSpec>>,
}

impl Validate for StaticFilesSpec {
    fn validate(&self, report: &mut Report) {
        for route in &self.routes {
            validate_static_route(&route.value, report);
        }
    }
}
