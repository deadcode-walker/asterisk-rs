//! endpoint query operations (read-only).

use crate::client::AriClient;
use crate::error::Result;

/// ari endpoint representation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Endpoint {
    pub technology: String,
    pub resource: String,
    #[serde(default)]
    pub state: Option<String>,
    #[serde(default)]
    pub channel_ids: Vec<String>,
}

/// list all endpoints
pub async fn list(client: &AriClient) -> Result<Vec<Endpoint>> {
    client.get("/endpoints").await
}

/// list endpoints for a specific technology
pub async fn list_by_tech(client: &AriClient, tech: &str) -> Result<Vec<Endpoint>> {
    client.get(&format!("/endpoints/{tech}")).await
}

/// get a specific endpoint
pub async fn get(client: &AriClient, tech: &str, resource: &str) -> Result<Endpoint> {
    client
        .get(&format!("/endpoints/{tech}/{resource}"))
        .await
}
