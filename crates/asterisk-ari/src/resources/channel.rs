//! channel operations — originate, answer, hangup, dtmf, hold, mute, etc.

use crate::client::AriClient;
use crate::error::Result;
use crate::event::{Channel, LiveRecording, Playback};

/// parameters for originating a new channel
#[derive(Debug, Clone, Default, serde::Serialize)]
pub struct OriginateParams {
    pub endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub app_args: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub caller_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub timeout: Option<i32>,
}

/// ari channel variable response
#[derive(Debug, Clone, serde::Deserialize)]
pub struct Variable {
    pub value: String,
}

/// handle to an ari channel, bundling channel id with client reference
#[derive(Debug, Clone)]
pub struct ChannelHandle {
    id: String,
    client: AriClient,
}

impl ChannelHandle {
    pub fn new(id: impl Into<String>, client: AriClient) -> Self {
        Self {
            id: id.into(),
            client,
        }
    }

    pub fn id(&self) -> &str {
        &self.id
    }

    /// answer the channel
    pub async fn answer(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/answer", self.id))
            .await
    }

    /// hang up the channel with an optional reason
    pub async fn hangup(&self, reason: Option<&str>) -> Result<()> {
        let path = match reason {
            Some(r) => format!("/channels/{}?reason={}", self.id, r),
            None => format!("/channels/{}", self.id),
        };
        self.client.delete(&path).await
    }

    /// play media on the channel
    pub async fn play(&self, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/channels/{}/play", self.id),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// start recording on the channel
    pub async fn record(&self, name: &str, format: &str) -> Result<LiveRecording> {
        self.client
            .post(
                &format!("/channels/{}/record", self.id),
                &serde_json::json!({"name": name, "format": format}),
            )
            .await
    }

    /// mute the channel, optionally specifying direction (both, in, out)
    pub async fn mute(&self, direction: Option<&str>) -> Result<()> {
        let path = match direction {
            Some(d) => format!("/channels/{}/mute?direction={}", self.id, d),
            None => format!("/channels/{}/mute", self.id),
        };
        self.client.post_empty(&path).await
    }

    /// unmute the channel, optionally specifying direction
    pub async fn unmute(&self, direction: Option<&str>) -> Result<()> {
        let path = match direction {
            Some(d) => format!("/channels/{}/mute?direction={}", self.id, d),
            None => format!("/channels/{}/mute", self.id),
        };
        self.client.delete(&path).await
    }

    /// place the channel on hold
    pub async fn hold(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/hold", self.id))
            .await
    }

    /// remove the channel from hold
    pub async fn unhold(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/hold", self.id))
            .await
    }

    /// send dtmf digits to the channel
    pub async fn send_dtmf(&self, dtmf: &str) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/dtmf?dtmf={}", self.id, dtmf))
            .await
    }

    /// get a channel variable
    pub async fn get_variable(&self, name: &str) -> Result<Variable> {
        self.client
            .get(&format!("/channels/{}/variable?variable={}", self.id, name))
            .await
    }

    /// set a channel variable
    pub async fn set_variable(&self, name: &str, value: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/variable?variable={}&value={}",
                self.id, name, value
            ))
            .await
    }

    /// continue the channel in the dialplan
    pub async fn continue_in_dialplan(
        &self,
        context: Option<&str>,
        extension: Option<&str>,
        priority: Option<i64>,
    ) -> Result<()> {
        let mut path = format!("/channels/{}/continue", self.id);
        let mut params = Vec::new();
        if let Some(c) = context {
            params.push(format!("context={c}"));
        }
        if let Some(e) = extension {
            params.push(format!("extension={e}"));
        }
        if let Some(p) = priority {
            params.push(format!("priority={p}"));
        }
        if !params.is_empty() {
            path.push('?');
            path.push_str(&params.join("&"));
        }
        self.client.post_empty(&path).await
    }

    /// snoop on the channel — spy and/or whisper
    pub async fn snoop(
        &self,
        spy: Option<&str>,
        whisper: Option<&str>,
        app: &str,
    ) -> Result<Channel> {
        let mut params = vec![format!("app={app}")];
        if let Some(s) = spy {
            params.push(format!("spy={s}"));
        }
        if let Some(w) = whisper {
            params.push(format!("whisper={w}"));
        }
        let query = params.join("&");
        self.client
            .post(
                &format!("/channels/{}/snoop?{}", self.id, query),
                &serde_json::json!({}),
            )
            .await
    }
}

/// list all active channels
pub async fn list(client: &AriClient) -> Result<Vec<Channel>> {
    client.get("/channels").await
}

/// get details for a specific channel
pub async fn get(client: &AriClient, channel_id: &str) -> Result<Channel> {
    client.get(&format!("/channels/{channel_id}")).await
}

/// originate a new channel
pub async fn originate(client: &AriClient, params: &OriginateParams) -> Result<Channel> {
    client.post("/channels", params).await
}
