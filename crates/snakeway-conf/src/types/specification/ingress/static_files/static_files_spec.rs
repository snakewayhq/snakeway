use crate::types::OriginDeprecated;
use crate::types::specification::ingress::static_files::static_route_spec::StaticRouteSpec;
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize, Default, Serialize)]
pub struct StaticFilesSpec {
    #[serde(skip)]
    pub origin: OriginDeprecated,
    pub routes: Vec<StaticRouteSpec>,
}
