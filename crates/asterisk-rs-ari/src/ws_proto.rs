//! shared protocol types for REST-over-WebSocket communication.

/// REST request envelope sent over websocket
#[derive(serde::Serialize)]
pub(crate) struct WsRestRequest {
    #[serde(rename = "type")]
    pub type_field: &'static str,
    pub request_id: String,
    pub method: String,
    pub uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub message_body: Option<String>,
}
