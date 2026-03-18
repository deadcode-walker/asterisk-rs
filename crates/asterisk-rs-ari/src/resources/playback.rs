//! playback control operations.

use crate::client::AriClient;
use crate::error::Result;
use crate::event::Playback;

/// handle to an ari playback
#[derive(Debug, Clone)]
pub struct PlaybackHandle {
    id: String,
    client: AriClient,
}

impl PlaybackHandle {
    pub fn new(id: impl Into<String>, client: AriClient) -> Self {
        Self {
            id: id.into(),
            client,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// control the playback (pause, unpause, restart, reverse, forward)
    pub async fn control(&self, operation: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/playbacks/{}/control?operation={}",
                self.id, operation
            ))
            .await
    }

    /// stop the playback
    pub async fn stop(&self) -> Result<()> {
        self.client.delete(&format!("/playbacks/{}", self.id)).await
    }

    /// get current playback state
    pub async fn get(&self) -> Result<Playback> {
        self.client.get(&format!("/playbacks/{}", self.id)).await
    }
}
