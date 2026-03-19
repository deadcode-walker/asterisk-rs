//! REST transport abstraction for HTTP and WebSocket modes.

use std::time::Duration;

use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::websocket::WsEventListener;
use crate::ws_transport::WsTransport;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventBus;

/// response from a transport REST operation
pub(crate) struct TransportResponse {
    pub status: u16,
    pub body: Option<String>,
}

/// internal transport implementation — dispatches REST calls to either
/// HTTP (reqwest) or a unified WebSocket connection
pub(crate) enum TransportInner {
    Http(HttpTransport),
    WebSocket(WsTransport),
}

impl TransportInner {
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<TransportResponse> {
        match self {
            Self::Http(t) => t.request(method, path, body).await,
            Self::WebSocket(t) => t.request(method, path, body).await,
        }
    }

    pub fn shutdown(&self) {
        match self {
            Self::Http(t) => t.ws_listener.shutdown(),
            Self::WebSocket(t) => t.shutdown(),
        }
    }
}

/// HTTP-based transport — uses reqwest for REST and a separate
/// WebSocket listener for events
pub(crate) struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    username: String,
    password: String,
    ws_listener: WsEventListener,
}

impl HttpTransport {
    pub fn new(
        base_url: &str,
        username: String,
        password: String,
        ws_url: String,
        event_bus: EventBus<AriMessage>,
        reconnect: ReconnectPolicy,
    ) -> Result<Self> {
        let client = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(AriError::Http)?;

        let ws_listener = WsEventListener::spawn(ws_url, event_bus, reconnect);

        Ok(Self {
            client,
            base_url: base_url.trim_end_matches('/').to_owned(),
            username,
            password,
            ws_listener,
        })
    }

    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<TransportResponse> {
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let http_method = parse_method(method)?;

        let mut req = self
            .client
            .request(http_method, &url)
            .basic_auth(&self.username, Some(&self.password));

        if let Some(json_body) = body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(json_body);
        }

        let response = req.send().await.map_err(AriError::Http)?;
        let status = response.status().as_u16();

        if response.status().is_client_error() || response.status().is_server_error() {
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read error body".to_owned());
            return Err(AriError::Api { status, message });
        }

        let text = response.text().await.map_err(AriError::Http)?;
        let body = if text.is_empty() { None } else { Some(text) };
        Ok(TransportResponse { status, body })
    }
}

fn parse_method(method: &str) -> Result<reqwest::Method> {
    match method {
        "GET" => Ok(reqwest::Method::GET),
        "POST" => Ok(reqwest::Method::POST),
        "PUT" => Ok(reqwest::Method::PUT),
        "DELETE" => Ok(reqwest::Method::DELETE),
        other => Err(AriError::WebSocket(format!(
            "unsupported HTTP method: {other}"
        ))),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_method_valid() {
        assert_eq!(
            parse_method("GET").expect("should parse GET"),
            reqwest::Method::GET
        );
        assert_eq!(
            parse_method("POST").expect("should parse POST"),
            reqwest::Method::POST
        );
        assert_eq!(
            parse_method("PUT").expect("should parse PUT"),
            reqwest::Method::PUT
        );
        assert_eq!(
            parse_method("DELETE").expect("should parse DELETE"),
            reqwest::Method::DELETE
        );
    }

    #[test]
    fn test_parse_method_invalid() {
        let err = parse_method("PATCH").expect_err("should reject PATCH");
        assert!(matches!(err, AriError::WebSocket(_)));
    }
}
