//! recording control operations — live and stored.

use crate::client::{url_encode, AriClient};
use crate::error::Result;
use crate::event::LiveRecording;

/// stored recording metadata
#[derive(Debug, Clone, serde::Deserialize)]
pub struct StoredRecording {
    pub name: String,
    pub format: String,
}

/// handle to a live ari recording
#[derive(Debug, Clone)]
pub struct RecordingHandle {
    name: String,
    client: AriClient,
}

impl RecordingHandle {
    pub fn new(name: impl Into<String>, client: AriClient) -> Self {
        Self {
            name: name.into(),
            client,
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    /// stop the live recording
    pub async fn stop(&self) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/recordings/live/{}/stop",
                url_encode(&self.name)
            ))
            .await
    }

    /// pause the live recording
    pub async fn pause(&self) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/recordings/live/{}/pause",
                url_encode(&self.name)
            ))
            .await
    }

    /// unpause the live recording
    pub async fn unpause(&self) -> Result<()> {
        self.client
            .delete(&format!(
                "/recordings/live/{}/pause",
                url_encode(&self.name)
            ))
            .await
    }

    /// mute the live recording
    pub async fn mute(&self) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/recordings/live/{}/mute",
                url_encode(&self.name)
            ))
            .await
    }

    /// unmute the live recording
    pub async fn unmute(&self) -> Result<()> {
        self.client
            .delete(&format!(
                "/recordings/live/{}/mute",
                url_encode(&self.name)
            ))
            .await
    }

    /// get current live recording state
    pub async fn get(&self) -> Result<LiveRecording> {
        self.client
            .get(&format!("/recordings/live/{}", url_encode(&self.name)))
            .await
    }
}

/// list all stored recordings
pub async fn list_stored(client: &AriClient) -> Result<Vec<StoredRecording>> {
    client.get("/recordings/stored").await
}
