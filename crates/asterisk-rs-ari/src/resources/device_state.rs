//! device state operations.

use crate::client::{AriClient, url_encode};
use crate::error::Result;

/// ari device state representation
#[derive(Debug, Clone, serde::Deserialize)]
#[non_exhaustive]
pub struct DeviceState {
    pub name: String,
    pub state: String,
}

/// list all device states
pub async fn list(client: &AriClient) -> Result<Vec<DeviceState>> {
    client.get("/deviceStates").await
}

/// get a specific device state
pub async fn get(client: &AriClient, name: &str) -> Result<DeviceState> {
    client
        .get(&format!("/deviceStates/{}", url_encode(name)))
        .await
}

/// update a device state
pub async fn update(client: &AriClient, name: &str, state: &str) -> Result<()> {
    client
        .put_empty(&format!(
            "/deviceStates/{}?deviceState={}",
            url_encode(name),
            url_encode(state)
        ))
        .await
}

/// delete a device state
pub async fn delete(client: &AriClient, name: &str) -> Result<()> {
    client
        .delete(&format!("/deviceStates/{}", url_encode(name)))
        .await
}
