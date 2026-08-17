//! bridge operations — create, destroy, add/remove channels, play, record.

use crate::client::{AriClient, url_encode};
use crate::error::Result;
use crate::event::{Bridge, LiveRecording, Playback};
use crate::resources::playback::PlaybackHandle;
use crate::resources::recording::RecordingHandle;

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
                url_encode(&self.id),
                url_encode(channel_id)
            ))
            .await
    }

    /// remove a channel from this bridge
    pub async fn remove_channel(&self, channel_id: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/bridges/{}/removeChannel?channel={}",
                url_encode(&self.id),
                url_encode(channel_id)
            ))
            .await
    }

    /// play media on the bridge
    pub async fn play(&self, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/bridges/{}/play", url_encode(&self.id)),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// Play media and return the lifecycle handle for the created playback.
    pub async fn play_handle(&self, media: &str) -> Result<PlaybackHandle> {
        let playback = self.play(media).await?;
        Ok(PlaybackHandle::new(playback.id, self.client.clone()))
    }

    /// start recording on the bridge
    pub async fn record(&self, name: &str, format: &str) -> Result<LiveRecording> {
        self.client
            .post(
                &format!("/bridges/{}/record", url_encode(&self.id)),
                &serde_json::json!({"name": name, "format": format}),
            )
            .await
    }

    /// Start recording and return the lifecycle handle for the created recording.
    pub async fn record_handle(&self, name: &str, format: &str) -> Result<RecordingHandle> {
        let recording = self.record(name, format).await?;
        Ok(RecordingHandle::new(recording.name, self.client.clone()))
    }

    /// destroy this bridge
    pub async fn destroy(&self) -> Result<()> {
        self.client
            .delete(&format!("/bridges/{}", url_encode(&self.id)))
            .await
    }

    /// start music on hold for the bridge
    pub async fn start_moh(&self, moh_class: Option<&str>) -> Result<()> {
        let path = match moh_class {
            Some(c) => format!(
                "/bridges/{}/moh?mohClass={}",
                url_encode(&self.id),
                url_encode(c)
            ),
            None => format!("/bridges/{}/moh", url_encode(&self.id)),
        };
        self.client.post_empty(&path).await
    }

    /// stop music on hold for the bridge
    pub async fn stop_moh(&self) -> Result<()> {
        self.client
            .delete(&format!("/bridges/{}/moh", url_encode(&self.id)))
            .await
    }

    /// play media with a specific playback id
    pub async fn play_with_id(&self, playback_id: &str, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!(
                    "/bridges/{}/play/{}",
                    url_encode(&self.id),
                    url_encode(playback_id)
                ),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// Play media with a caller-selected ID and return its lifecycle handle.
    pub async fn play_with_id_handle(
        &self,
        playback_id: &str,
        media: &str,
    ) -> Result<PlaybackHandle> {
        self.play_with_id(playback_id, media).await?;
        Ok(PlaybackHandle::new(playback_id, self.client.clone()))
    }

    /// set the video source for the bridge
    pub async fn set_video_source(&self, channel_id: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/bridges/{}/videoSource/{}",
                url_encode(&self.id),
                url_encode(channel_id)
            ))
            .await
    }

    /// clear the video source for the bridge
    pub async fn clear_video_source(&self) -> Result<()> {
        self.client
            .delete(&format!("/bridges/{}/videoSource", url_encode(&self.id)))
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

/// Create a bridge and return its lifecycle handle.
pub async fn create_handle(
    client: &AriClient,
    bridge_type: Option<&str>,
    name: Option<&str>,
) -> Result<BridgeHandle> {
    let bridge = create(client, bridge_type, name).await?;
    Ok(BridgeHandle::new(bridge.id, client.clone()))
}

/// list all bridges
pub async fn list(client: &AriClient) -> Result<Vec<Bridge>> {
    client.get("/bridges").await
}

/// get details for a specific bridge
pub async fn get(client: &AriClient, bridge_id: &str) -> Result<Bridge> {
    client
        .get(&format!("/bridges/{}", url_encode(bridge_id)))
        .await
}
