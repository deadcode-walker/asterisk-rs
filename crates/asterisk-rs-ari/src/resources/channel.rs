//! channel operations — originate, answer, hangup, dtmf, hold, mute, etc.

use std::collections::HashMap;

use crate::client::{AriClient, url_encode};
use crate::error::Result;
use crate::event::{Channel, LiveRecording, Playback};

/// parameters for originating a new channel
#[derive(Debug, Clone, serde::Serialize)]
#[must_use]
#[non_exhaustive]
pub struct OriginateParams {
    endpoint: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    extension: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    context: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    priority: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    app: Option<String>,
    #[serde(rename = "appArgs", skip_serializing_if = "Option::is_none")]
    app_args: Option<String>,
    #[serde(rename = "callerId", skip_serializing_if = "Option::is_none")]
    caller_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    timeout: Option<i32>,
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    pub(crate) channel_id: Option<String>,
    #[serde(rename = "otherChannelId", skip_serializing_if = "Option::is_none")]
    other_channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    originator: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    formats: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<HashMap<String, String>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    label: Option<String>,
}

impl OriginateParams {
    /// Create originate parameters with the required endpoint.
    pub fn new(endpoint: impl Into<String>) -> Self {
        Self {
            endpoint: endpoint.into(),
            extension: None,
            context: None,
            priority: None,
            app: None,
            app_args: None,
            caller_id: None,
            timeout: None,
            channel_id: None,
            other_channel_id: None,
            originator: None,
            formats: None,
            variables: None,
            label: None,
        }
    }

    pub fn extension(mut self, value: impl Into<String>) -> Self {
        self.extension = Some(value.into());
        self
    }
    pub fn context(mut self, value: impl Into<String>) -> Self {
        self.context = Some(value.into());
        self
    }
    pub fn priority(mut self, value: i64) -> Self {
        self.priority = Some(value);
        self
    }
    pub fn app(mut self, value: impl Into<String>) -> Self {
        self.app = Some(value.into());
        self
    }
    pub fn app_args(mut self, value: impl Into<String>) -> Self {
        self.app_args = Some(value.into());
        self
    }
    pub fn caller_id(mut self, value: impl Into<String>) -> Self {
        self.caller_id = Some(value.into());
        self
    }
    pub fn timeout(mut self, value: i32) -> Self {
        self.timeout = Some(value);
        self
    }
    pub fn channel_id(mut self, value: impl Into<String>) -> Self {
        self.channel_id = Some(value.into());
        self
    }
    pub fn other_channel_id(mut self, value: impl Into<String>) -> Self {
        self.other_channel_id = Some(value.into());
        self
    }
    pub fn originator(mut self, value: impl Into<String>) -> Self {
        self.originator = Some(value.into());
        self
    }
    pub fn formats(mut self, value: impl Into<String>) -> Self {
        self.formats = Some(value.into());
        self
    }
    pub fn variables(mut self, value: HashMap<String, String>) -> Self {
        self.variables = Some(value);
        self
    }
    pub fn label(mut self, value: impl Into<String>) -> Self {
        self.label = Some(value.into());
        self
    }
}

/// parameters for starting an external media session
#[derive(Debug, Clone, serde::Serialize)]
#[must_use]
#[non_exhaustive]
pub struct ExternalMediaParams {
    app: String,
    external_host: String,
    format: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    encapsulation: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    connection_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    direction: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    data: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    transport_data: Option<String>,
    #[serde(rename = "channelId", skip_serializing_if = "Option::is_none")]
    channel_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    variables: Option<HashMap<String, String>>,
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
            data: None,
            transport_data: None,
            channel_id: None,
            variables: None,
        }
    }

    /// create params for chan_websocket's JSON control protocol
    ///
    /// Asterisk defaults chan_websocket control messages to plaintext. This
    /// constructor selects the JSON protocol required by [`crate::media::MediaChannel`].
    pub fn websocket_json(
        app: impl Into<String>,
        external_host: impl Into<String>,
        format: impl Into<String>,
    ) -> Self {
        Self::new(app, external_host, format)
            .encapsulation("none")
            .transport("websocket")
            .transport_data("f(json)")
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

    /// set the arbitrary data passed to the external-media channel
    pub fn data(mut self, data: impl Into<String>) -> Self {
        self.data = Some(data.into());
        self
    }

    /// set transport-specific dial-string data
    ///
    /// For chan_websocket, use `f(json)` to select JSON control messages.
    pub fn transport_data(mut self, transport_data: impl Into<String>) -> Self {
        self.transport_data = Some(transport_data.into());
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
#[non_exhaustive]
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
            .post_empty(&format!("/channels/{}/answer", url_encode(&self.id)))
            .await
    }

    /// hang up the channel with an optional reason
    pub async fn hangup(&self, reason: Option<&str>) -> Result<()> {
        let path = match reason {
            Some(r) => format!(
                "/channels/{}?reason={}",
                url_encode(&self.id),
                url_encode(r)
            ),
            None => format!("/channels/{}", url_encode(&self.id)),
        };
        self.client.delete(&path).await
    }

    /// play media on the channel
    pub async fn play(&self, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!("/channels/{}/play", url_encode(&self.id)),
                &serde_json::json!({"media": media}),
            )
            .await
    }

    /// start recording on the channel
    pub async fn record(&self, name: &str, format: &str) -> Result<LiveRecording> {
        self.client
            .post(
                &format!("/channels/{}/record", url_encode(&self.id)),
                &serde_json::json!({"name": name, "format": format}),
            )
            .await
    }

    /// mute the channel, optionally specifying direction (both, in, out)
    pub async fn mute(&self, direction: Option<&str>) -> Result<()> {
        let path = match direction {
            Some(d) => format!(
                "/channels/{}/mute?direction={}",
                url_encode(&self.id),
                url_encode(d)
            ),
            None => format!("/channels/{}/mute", url_encode(&self.id)),
        };
        self.client.post_empty(&path).await
    }

    /// unmute the channel, optionally specifying direction
    pub async fn unmute(&self, direction: Option<&str>) -> Result<()> {
        let path = match direction {
            Some(d) => format!(
                "/channels/{}/mute?direction={}",
                url_encode(&self.id),
                url_encode(d)
            ),
            None => format!("/channels/{}/mute", url_encode(&self.id)),
        };
        self.client.delete(&path).await
    }

    /// place the channel on hold
    pub async fn hold(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/hold", url_encode(&self.id)))
            .await
    }

    /// remove the channel from hold
    pub async fn unhold(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/hold", url_encode(&self.id)))
            .await
    }

    /// send dtmf digits to the channel
    pub async fn send_dtmf(&self, dtmf: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/dtmf?dtmf={}",
                url_encode(&self.id),
                url_encode(dtmf)
            ))
            .await
    }

    /// get a channel variable
    pub async fn get_variable(&self, name: &str) -> Result<Variable> {
        self.client
            .get(&format!(
                "/channels/{}/variable?variable={}",
                url_encode(&self.id),
                url_encode(name)
            ))
            .await
    }

    /// set a channel variable
    pub async fn set_variable(&self, name: &str, value: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/variable?variable={}&value={}",
                url_encode(&self.id),
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
        let mut path = format!("/channels/{}/continue", url_encode(&self.id));
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
                &format!("/channels/{}/snoop?{}", url_encode(&self.id), query),
                &serde_json::json!({}),
            )
            .await
    }

    /// redirect the channel to a new endpoint
    pub async fn redirect(&self, endpoint: &str) -> Result<()> {
        self.client
            .post_empty(&format!(
                "/channels/{}/redirect?endpoint={}",
                url_encode(&self.id),
                url_encode(endpoint),
            ))
            .await
    }

    /// start ringing on the channel
    pub async fn ring(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/ring", url_encode(&self.id)))
            .await
    }

    /// stop ringing on the channel
    pub async fn ring_stop(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/ring", url_encode(&self.id)))
            .await
    }

    /// start silence on the channel
    pub async fn start_silence(&self) -> Result<()> {
        self.client
            .post_empty(&format!("/channels/{}/silence", url_encode(&self.id)))
            .await
    }

    /// stop silence on the channel
    pub async fn stop_silence(&self) -> Result<()> {
        self.client
            .delete(&format!("/channels/{}/silence", url_encode(&self.id)))
            .await
    }

    /// play media on the channel with additional options
    pub async fn play_with_id(&self, playback_id: &str, media: &str) -> Result<Playback> {
        self.client
            .post(
                &format!(
                    "/channels/{}/play/{}",
                    url_encode(&self.id),
                    url_encode(playback_id)
                ),
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
            .post_empty(&format!("/channels/{}/dial{}", url_encode(&self.id), query))
            .await
    }

    /// get rtp statistics for the channel
    pub async fn rtp_statistics(&self) -> Result<serde_json::Value> {
        self.client
            .get(&format!(
                "/channels/{}/rtp_statistics",
                url_encode(&self.id)
            ))
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
    client
        .get(&format!("/channels/{}", url_encode(channel_id)))
        .await
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
