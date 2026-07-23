use super::static_route_spec::StaticRouteSpec;
use confval::prelude::{Located, Report, Validate};
use serde::Serialize;

#[derive(Debug, Serialize, Default, confval::Spec)]
pub struct StaticFilesSpec {
    #[confval(nested)]
    pub routes: Vec<Located<StaticRouteSpec>>,
}

impl Validate for StaticFilesSpec {
    fn validate(&self, _report: &mut Report) {}
}
