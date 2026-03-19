//! channel operations — originate, answer, hangup, dtmf, hold, mute, etc.

use std::collections::HashMap;

use crate::client::{url_encode, AriClient};
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

    pub fn encapsulation(mut self, encapsulation: impl Into<String>) -> Self {
        self.encapsulation = Some(encapsulation.into());
        self
    }

    pub fn transport(mut self, transport: impl Into<String>) -> Self {
        self.transport = Some(transport.into());
        self
    }

    pub fn connection_type(mut self, connection_type: impl Into<String>) -> Self {
        self.connection_type = Some(connection_type.into());
        self
    }

    pub fn direction(mut self, direction: impl Into<String>) -> Self {
        self.direction = Some(direction.into());
        self
    }

    pub fn channel_id(mut self, channel_id: impl Into<String>) -> Self {
        self.channel_id = Some(channel_id.into());
        self
    }

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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_external_media_params_new() {
        let params = ExternalMediaParams::new("myapp", "192.168.1.1:8000", "ulaw");
        assert_eq!(params.app, "myapp");
        assert_eq!(params.external_host, "192.168.1.1:8000");
        assert_eq!(params.format, "ulaw");
        assert!(params.encapsulation.is_none());
        assert!(params.transport.is_none());
        assert!(params.connection_type.is_none());
        assert!(params.direction.is_none());
        assert!(params.channel_id.is_none());
        assert!(params.variables.is_none());
    }

    #[test]
    fn test_external_media_params_builder() {
        let vars = HashMap::from([("key".to_string(), "val".to_string())]);
        let params = ExternalMediaParams::new("app", "host:1234", "slin16")
            .encapsulation("rtp")
            .transport("udp")
            .connection_type("client")
            .direction("both")
            .channel_id("chan-123")
            .variables(vars.clone());
        assert_eq!(params.encapsulation.as_deref(), Some("rtp"));
        assert_eq!(params.transport.as_deref(), Some("udp"));
        assert_eq!(params.connection_type.as_deref(), Some("client"));
        assert_eq!(params.direction.as_deref(), Some("both"));
        assert_eq!(params.channel_id.as_deref(), Some("chan-123"));
        assert_eq!(params.variables, Some(vars));
    }

    #[test]
    fn test_external_media_params_serialization() {
        let params = ExternalMediaParams::new("app", "host:1234", "ulaw")
            .channel_id("ext-1");
        let json = serde_json::to_value(&params).expect("serialization should succeed");
        assert_eq!(json["app"], "app");
        assert_eq!(json["external_host"], "host:1234");
        assert_eq!(json["format"], "ulaw");
        assert_eq!(json["channelId"], "ext-1");
        // optional fields omitted when none
        assert!(json.get("encapsulation").is_none());
        assert!(json.get("transport").is_none());
        assert!(json.get("variables").is_none());
    }

    #[test]
    fn test_originate_params_new_fields() {
        let vars = HashMap::from([("CALLERID(name)".to_string(), "Test".to_string())]);
        let params = OriginateParams {
            endpoint: "PJSIP/100".to_string(),
            channel_id: Some("chan-orig".to_string()),
            other_channel_id: Some("chan-other".to_string()),
            originator: Some("orig-chan".to_string()),
            formats: Some("ulaw,alaw".to_string()),
            variables: Some(vars),
            label: Some("my-label".to_string()),
            ..Default::default()
        };
        let json = serde_json::to_value(&params).expect("serialization should succeed");
        assert_eq!(json["channelId"], "chan-orig");
        assert_eq!(json["otherChannelId"], "chan-other");
        assert_eq!(json["originator"], "orig-chan");
        assert_eq!(json["formats"], "ulaw,alaw");
        assert_eq!(json["variables"]["CALLERID(name)"], "Test");
        assert_eq!(json["label"], "my-label");
    }

    #[test]
    fn test_originate_params_skip_none() {
        let params = OriginateParams {
            endpoint: "PJSIP/200".to_string(),
            ..Default::default()
        };
        let json = serde_json::to_value(&params).expect("serialization should succeed");
        assert_eq!(json["endpoint"], "PJSIP/200");
        // all optional fields should be absent
        assert!(json.get("channelId").is_none());
        assert!(json.get("otherChannelId").is_none());
        assert!(json.get("originator").is_none());
        assert!(json.get("formats").is_none());
        assert!(json.get("variables").is_none());
        assert!(json.get("label").is_none());
        assert!(json.get("extension").is_none());
        assert!(json.get("timeout").is_none());
    }
}
