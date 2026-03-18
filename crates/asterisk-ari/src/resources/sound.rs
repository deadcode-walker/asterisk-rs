//! sound query operations (read-only).

use crate::client::AriClient;
use crate::error::Result;

/// format information for a sound
#[derive(Debug, Clone, serde::Deserialize)]
pub struct SoundFormat {
    pub language: String,
    pub format: String,
}

/// ari sound representation
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Sound {
    pub id: String,
    #[serde(default)]
    pub text: String,
    #[serde(default)]
    pub formats: Vec<SoundFormat>,
}

/// list all sounds
pub async fn list(client: &AriClient) -> Result<Vec<Sound>> {
    client.get("/sounds").await
}

/// get a specific sound
pub async fn get(client: &AriClient, sound_id: &str) -> Result<Sound> {
    client.get(&format!("/sounds/{sound_id}")).await
}
