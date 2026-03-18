//! stasis application management.

use crate::client::{url_encode, AriClient};
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

/// subscribe an application to an event source
pub async fn subscribe(
    client: &AriClient,
    application_name: &str,
    event_source: &str,
) -> Result<Application> {
    client
        .post(
            &format!(
                "/applications/{application_name}/subscription?eventSource={}",
                url_encode(event_source)
            ),
            &serde_json::json!({}),
        )
        .await
}

/// unsubscribe an application from an event source
pub async fn unsubscribe(
    client: &AriClient,
    application_name: &str,
    event_source: &str,
) -> Result<Application> {
    client
        .delete_with_response(&format!(
            "/applications/{application_name}/subscription?eventSource={}",
            url_encode(event_source)
        ))
        .await
}

/// set the event filter for an application
pub async fn set_event_filter(
    client: &AriClient,
    application_name: &str,
    filter: &serde_json::Value,
) -> Result<Application> {
    client
        .put(
            &format!("/applications/{application_name}/eventFilter"),
            filter,
        )
        .await
}
