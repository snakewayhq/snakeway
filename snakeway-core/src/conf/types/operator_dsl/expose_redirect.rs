use serde::Deserialize;

#[derive(Debug, Deserialize, Default)]
pub struct ExposeRedirectConfig {
    pub addr: String,
    pub to: String,
    pub status: u16,
}
