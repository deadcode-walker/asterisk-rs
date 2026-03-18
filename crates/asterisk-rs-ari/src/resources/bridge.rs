//! bridge operations — create, destroy, add/remove channels, play, record.

use crate::client::{url_encode, AriClient};
use crate::error::Result;
use crate::event::{Bridge, LiveRecording, Playback};

/// handle to an ari bridge
#[derive(Debug, Clone)]
pub struct BridgeHandle {
    id: String,
    client: AriClient,
}

impl BridgeHandle {
    pub fn new(id: impl Into<String>, client: AriClient) -> Self {
        Self {
            id: id.into(),
            client,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// add a channel to this bridge
    pub async fn add_channel(&self, channel_id: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/bridges/{}/addChannel?channel={}",
                self.id,
                url_encode(channel_id)
            ))
            .await
    }

    /// remove a channel from this bridge
    pub async fn remove_channel(&self, channel_id: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/bridges/{}/removeChannel?channel={}",
                self.id,
                url_encode(channel_id)
            ))
            .await
    }

    /// play media on the bridge
    pub async fn play(&self, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/bridges/{}/play", self.id),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// start recording on the bridge
    pub async fn record(&self, name: &str, format: &str) -> Result<LiveRecording> {
        self.client
            .post(
                &format!("/bridges/{}/record", self.id),
                &serde_json::json!({"name": name, "format": format}),
            )
            .await
    }

    /// destroy this bridge
    pub async fn destroy(&self) -> Result<()> {
        self.client.delete(&format!("/bridges/{}", self.id)).await
    }

    /// start music on hold for the bridge
    pub async fn start_moh(&self, moh_class: Option<&str>) -> Result<()> {
        let path = match moh_class {
            Some(c) => format!("/bridges/{}/moh?mohClass={}", self.id, url_encode(c)),
            None => format!("/bridges/{}/moh", self.id),
        };
        self.client.post_empty(&path).await
    }

    /// stop music on hold for the bridge
    pub async fn stop_moh(&self) -> Result<()> {
        self.client
            .delete(&format!("/bridges/{}/moh", self.id))
            .await
    }

    /// play media with a specific playback id
    pub async fn play_with_id(&self, playback_id: &str, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/bridges/{}/play/{}", self.id, playback_id),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// set the video source for the bridge
    pub async fn set_video_source(&self, channel_id: &str) -> Result<()> {
        self.client
            .post_empty(&format!("/bridges/{}/videoSource/{}", self.id, channel_id))
            .await
    }

    /// clear the video source for the bridge
    pub async fn clear_video_source(&self) -> Result<()> {
        self.client
            .delete(&format!("/bridges/{}/videoSource", self.id))
            .await
    }
}

/// create a new bridge
pub async fn create(
    client: &AriClient,
    bridge_type: Option<&str>,
    name: Option<&str>,
) -> Result<Bridge> {
    // build json body, skipping none fields
    let mut body = serde_json::Map::new();
    if let Some(t) = bridge_type {
        body.insert("type".to_owned(), serde_json::Value::String(t.to_owned()));
    }
    if let Some(n) = name {
        body.insert("name".to_owned(), serde_json::Value::String(n.to_owned()));
    }
    client
        .post("/bridges", &serde_json::Value::Object(body))
        .await
}

/// list all bridges
pub async fn list(client: &AriClient) -> Result<Vec<Bridge>> {
    client.get("/bridges").await
}

/// get details for a specific bridge
pub async fn get(client: &AriClient, bridge_id: &str) -> Result<Bridge> {
    client.get(&format!("/bridges/{bridge_id}")).await
}
