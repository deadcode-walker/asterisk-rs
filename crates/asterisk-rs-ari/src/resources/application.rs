//! stasis application management.

use crate::client::AriClient;
use crate::error::Result;

/// ari stasis application representation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Application {
    pub name: String,
    #[serde(default)]
    pub channel_ids: Vec<String>,
    #[serde(default)]
    pub bridge_ids: Vec<String>,
    #[serde(default)]
    pub endpoint_ids: Vec<String>,
    #[serde(default)]
    pub device_names: Vec<String>,
}

/// list all stasis applications
pub async fn list(client: &AriClient) -> Result<Vec<Application>> {
    client.get("/applications").await
}

/// get a specific stasis application
pub async fn get(client: &AriClient, app_name: &str) -> Result<Application> {
    client.get(&format!("/applications/{app_name}")).await
}
