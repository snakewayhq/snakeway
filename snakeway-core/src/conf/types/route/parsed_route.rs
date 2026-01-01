use serde::Deserialize;

#[derive(Debug, Clone, Deserialize)]
pub struct ParsedRoute {
    /// The explicit type of the route, to ease route validation.
    pub r#type: ParsedRouteType,

    /// The path to the
    pub path: String,

    /// Mutually exclusive with file_dir
    #[serde(skip_serializing_if = "Option::is_none")]
    pub service: Option<String>,

    /// Only valid for upstream services.
    #[serde(default)]
    pub allow_websocket: bool,

    /// Only valid for upstream services.
    #[serde(default)]
    pub ws_idle_timeout_ms: Option<u64>,

    /// Only valid for upstream services.
    #[serde(default)]
    pub ws_max_connections: Option<u64>,

    /// Mutually exclusive with service (validated later)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub file_dir: Option<String>,

    /// Only valid for static routes
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<String>,

    /// Only valid for static routes
    #[serde(default)]
    pub directory_listing: bool,
}

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ParsedRouteType {
    Service,
    Static,
}
