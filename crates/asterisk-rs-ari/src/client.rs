//! Main ARI client combining REST API and WebSocket events.

use std::sync::Arc;

use asterisk_rs_core::event::{EventBus, EventSubscription};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::AriConfig;
use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::websocket::WsEventListener;

/// async client for the Asterisk REST Interface
///
/// combines an HTTP client for REST operations with a background
/// websocket listener for receiving Stasis events
#[derive(Clone)]
pub struct AriClient {
    http: reqwest::Client,
    config: Arc<AriConfig>,
    event_bus: EventBus<AriMessage>,
    ws_listener: Arc<WsEventListener>,
}

impl std::fmt::Debug for AriClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriClient")
            .field("base_url", &self.config.base_url)
            .finish_non_exhaustive()
    }
}

impl AriClient {
    /// connect to an ARI server
    ///
    /// builds the HTTP client and spawns the websocket event listener
    pub async fn connect(config: AriConfig) -> Result<Self> {
        let http = reqwest::Client::builder()
            .connect_timeout(std::time::Duration::from_secs(10))
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(AriError::Http)?;

        let event_bus = EventBus::new(256);

        let ws_listener = WsEventListener::spawn(
            config.ws_url.to_string(),
            event_bus.clone(),
            config.reconnect_policy.clone(),
        );

        Ok(Self {
            http,
            config: Arc::new(config),
            event_bus,
            ws_listener: Arc::new(ws_listener),
        })
    }

    /// send a GET request to the given ARI path
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .get(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        Self::check_response(response)
            .await?
            .json()
            .await
            .map_err(AriError::Http)
    }

    /// send a POST request with a JSON body to the given ARI path
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .post(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .json(body)
            .send()
            .await?;

        Self::check_response(response)
            .await?
            .json()
            .await
            .map_err(AriError::Http)
    }

    /// send a POST request with no body to the given ARI path
    pub async fn post_empty(&self, path: &str) -> Result<()> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .post(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        Self::check_response(response).await?;
        Ok(())
    }

    /// send a PUT request with a JSON body to the given ARI path
    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .put(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .json(body)
            .send()
            .await?;

        Self::check_response(response)
            .await?
            .json()
            .await
            .map_err(AriError::Http)
    }

    /// send a PUT request with no body to the given ARI path
    pub async fn put_empty(&self, path: &str) -> Result<()> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .put(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        Self::check_response(response).await?;
        Ok(())
    }

    /// send a DELETE request to the given ARI path
    pub async fn delete(&self, path: &str) -> Result<()> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .delete(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        Self::check_response(response).await?;
        Ok(())
    }

    /// send a DELETE request and deserialize the response body
    pub async fn delete_with_response<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let url = self.build_url(path)?;
        let response = self
            .http
            .delete(url)
            .basic_auth(&self.config.username, Some(&self.config.password))
            .send()
            .await?;

        Self::check_response(response)
            .await?
            .json()
            .await
            .map_err(AriError::Http)
    }

    /// subscribe to ARI events from the websocket stream
    pub fn subscribe(&self) -> EventSubscription<AriMessage> {
        self.event_bus.subscribe()
    }

    /// subscribe to events matching a filter predicate
    pub fn subscribe_filtered(
        &self,
        predicate: impl Fn(&AriMessage) -> bool + Send + 'static,
    ) -> asterisk_rs_core::event::FilteredSubscription<AriMessage> {
        self.event_bus.subscribe_filtered(predicate)
    }

    /// access the underlying event bus
    pub fn events(&self) -> &EventBus<AriMessage> {
        &self.event_bus
    }

    /// shut down the websocket listener
    pub fn disconnect(&self) {
        self.ws_listener.shutdown();
    }

    /// build a full URL from a relative ARI path
    fn build_url(&self, path: &str) -> Result<String> {
        // path should be like "channels" or "bridges/bridge-id"
        let base = self.config.base_url.as_str().trim_end_matches('/');
        let path = path.trim_start_matches('/');
        Ok(format!("{base}/{path}"))
    }

    /// check response status, converting 4xx/5xx to AriError::Api
    async fn check_response(response: reqwest::Response) -> Result<reqwest::Response> {
        let status = response.status();
        if status.is_client_error() || status.is_server_error() {
            let status_code = status.as_u16();
            // try to read error body for context
            let message = response
                .text()
                .await
                .unwrap_or_else(|_| "failed to read error body".to_owned());
            return Err(AriError::Api {
                status: status_code,
                message,
            });
        }
        Ok(response)
    }
}

/// percent-encode a string for use in URL path segments or query values
pub fn url_encode(input: &str) -> String {
    let mut encoded = String::with_capacity(input.len());
    for byte in input.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'_' | b'.' | b'~' => {
                encoded.push(byte as char);
            }
            _ => {
                encoded.push_str(&format!("%{byte:02X}"));
            }
        }
    }
    encoded
}
