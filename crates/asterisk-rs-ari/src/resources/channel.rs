//! channel operations — originate, answer, hangup, dtmf, hold, mute, etc.

use std::collections::HashMap;

use crate::client::{url_encode, AriClient};
use crate::error::Result;
use crate::event::{Channel, LiveRecording, Playback};

/// parameters for originating a new channel
#[derive(Debug, Clone, Default, serde::Serialize)]
#[must_use]
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
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(rename = "otherChannelId", skip_serializing_if = "Option::is_none")]
    pub other_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub originator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub formats: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub label: Option<String>,
}

/// parameters for starting an external media session
#[derive(Debug, Clone, serde::Serialize)]
#[must_use]
pub struct ExternalMediaParams {
    pub app: String,
    pub external_host: String,
    pub format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub encapsulation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub transport: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub connection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub direction: Option<String>,
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub variables: Option<HashMap<String, String>>,
}

impl ExternalMediaParams {
    /// create params with required fields; optional fields default to none
    pub fn new(
        app: impl Into<String>,
        external_host: impl Into<String>,
        format: impl Into<String>,
    ) -> Self {
        Self {
            app: app.into(),
            external_host: external_host.into(),
            format: format.into(),
            encapsulation: None,
            transport: None,
            connection_type: None,
            direction: None,
            channel_id: None,
            variables: None,
        }
    }

    /// set the encapsulation type (e.g. `rtp`)
    pub fn encapsulation(mut self, encapsulation: impl Into<String>) -> Self {
        self.encapsulation = Some(encapsulation.into());
        self
    }

    /// set the transport protocol (e.g. `udp`)
    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
    }

    /// set the connection type
    pub fn connection_type(mut self, connection_type: impl Into<String>) -> Self {
        self.connection_type = Some(connection_type.into());
        self
    }

    /// set the media direction (e.g. `both`, `in`, `out`)
    pub fn direction(mut self, direction: impl Into<String>) -> Self {
        self.direction = Some(direction.into());
        self
    }

    /// set a specific channel id
    pub fn channel_id(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }

    /// set channel variables
    pub fn variables(mut self, variables: HashMap<String, String>) -> Self {
        self.variables = Some(variables);
        self
    }
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
    /// create a channel handle for the given id
    pub fn new(id: impl Into<String>, client: AriClient) -> Self {
        Self {
            id: id.into(),
            client,
        }
    }

    /// channel id
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
            Some(r) => format!("/channels/{}?reason={}", self.id, url_encode(r)),
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
            Some(d) => format!("/channels/{}/mute?direction={}", self.id, url_encode(d)),
            None => format!("/channels/{}/mute", self.id),
        };
        self.client.post_empty(&path).await
    }

    /// unmute the channel, optionally specifying direction
    pub async fn unmute(&self, direction: Option<&str>) -> Result<()> {
        let path = match direction {
            Some(d) => format!("/channels/{}/mute?direction={}", self.id, url_encode(d)),
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
            .post_empty(&format!(
                "/channels/{}/dtmf?dtmf={}",
                self.id,
                url_encode(dtmf)
            ))
            .await
    }

    /// get a channel variable
    pub async fn get_variable(&self, name: &str) -> Result<Variable> {
        self.client
            .get(&format!(
                "/channels/{}/variable?variable={}",
                self.id,
                url_encode(name)
            ))
            .await
    }

    /// set a channel variable
    pub async fn set_variable(&self, name: &str, value: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/variable?variable={}&value={}",
                self.id,
                url_encode(name),
                url_encode(value)
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
            params.push(format!("context={}", url_encode(c)));
        }
        if let Some(e) = extension {
            params.push(format!("extension={}", url_encode(e)));
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
        let mut params = vec![format!("app={}", url_encode(app))];
        if let Some(s) = spy {
            params.push(format!("spy={}", url_encode(s)));
        }
        if let Some(w) = whisper {
            params.push(format!("whisper={}", url_encode(w)));
        }
        let query = params.join("&");
        self.client
            .post(
                &format!("/channels/{}/snoop?{}", self.id, query),
                &serde_json::json!({}),
            )
            .await
    }

    /// redirect the channel to a different dialplan location
    pub async fn redirect(&self, context: &str, extension: &str, priority: i64) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/redirect?context={}&extension={}&priority={}",
                self.id,
                url_encode(context),
                url_encode(extension),
                priority
            ))
            .await
    }

    /// start ringing on the channel
    pub async fn ring(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/ring", self.id))
            .await
    }

    /// stop ringing on the channel
    pub async fn ring_stop(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/ring", self.id))
            .await
    }

    /// start silence on the channel
    pub async fn start_silence(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/silence", self.id))
            .await
    }

    /// stop silence on the channel
    pub async fn stop_silence(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/silence", self.id))
            .await
    }

    /// play media on the channel with additional options
    pub async fn play_with_id(&self, playback_id: &str, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/channels/{}/play/{}", self.id, playback_id),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// dial a created channel
    pub async fn dial(&self, caller: Option<&str>, timeout: Option<i32>) -> Result<()> {
        let mut params = Vec::new();
        if let Some(c) = caller {
            params.push(format!("caller={}", url_encode(c)));
        }
        if let Some(t) = timeout {
            params.push(format!("timeout={t}"));
        }
        let query = if params.is_empty() {
            String::new()
        } else {
            format!("?{}", params.join("&"))
        };
        self.client
            .post_empty(&format!("/channels/{}/dial{}", self.id, query))
            .await
    }

    /// get rtp statistics for the channel
    pub async fn rtp_statistics(&self) -> Result<serde_json::Value> {
        self.client
            .get(&format!("/channels/{}/rtp_statistics", self.id))
            .await
    }

    /// start an external media session
    pub async fn external_media(&self, params: &ExternalMediaParams) -> Result<Channel> {
        self.client.post("/channels/externalMedia", params).await
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

/// create a channel without dialing it
pub async fn create(client: &AriClient, endpoint: &str, app: &str) -> Result<Channel> {
    client
        .post(
            "/channels/create",
            &serde_json::json!({
                "endpoint": endpoint,
                "app": app,
            }),
        )
        .await
}

/// start an external media session
pub async fn external_media(client: &AriClient, params: &ExternalMediaParams) -> Result<Channel> {
    client.post("/channels/externalMedia", params).await
}
