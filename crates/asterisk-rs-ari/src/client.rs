//! Main ARI client combining REST API and WebSocket events.

use std::sync::Arc;

use asterisk_rs_core::event::{EventBus, EventSubscription};
use serde::de::DeserializeOwned;
use serde::Serialize;

use crate::config::{AriConfig, TransportMode};
use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::transport::{HttpTransport, TransportInner};
use crate::ws_transport::WsTransport;

/// async client for the Asterisk REST Interface
///
/// combines REST operations with a background websocket listener for
/// receiving Stasis events. supports both HTTP and unified WebSocket
/// transport modes.
#[derive(Clone)]
pub struct AriClient {
    transport: Arc<TransportInner>,
    config: Arc<AriConfig>,
    event_bus: EventBus<AriMessage>,
}

impl std::fmt::Debug for AriClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriClient")
            .field("base_url", &self.config.base_url)
            .field("transport_mode", &self.config.transport_mode)
            .finish_non_exhaustive()
    }
}

impl AriClient {
    /// connect to an ARI server
    ///
    /// builds the transport layer and spawns the websocket event listener.
    /// the transport mode is determined by [`AriConfig::transport_mode`].
    pub async fn connect(config: AriConfig) -> Result<Self> {
        let event_bus = EventBus::new(256);

        let transport = match config.transport_mode {
            TransportMode::Http => {
                let http = HttpTransport::new(
                    config.base_url.as_str(),
                    config.username.clone(),
                    config.password.clone(),
                    config.ws_url.to_string(),
                    event_bus.clone(),
                    config.reconnect_policy.clone(),
                )?;
                TransportInner::Http(http)
            }
            TransportMode::WebSocket => {
                let ws = WsTransport::spawn(
                    config.ws_url.to_string(),
                    event_bus.clone(),
                    config.reconnect_policy.clone(),
                );
                TransportInner::WebSocket(ws)
            }
        };

        Ok(Self {
            transport: Arc::new(transport),
            config: Arc::new(config),
            event_bus,
        })
    }

    /// send a GET request to the given ARI path
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.transport.request("GET", path, None).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a POST request with a JSON body to the given ARI path
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let json = serde_json::to_string(body).map_err(AriError::Json)?;
        let resp = self.transport.request("POST", path, Some(json)).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a POST request with no body to the given ARI path
    pub async fn post_empty(&self, path: &str) -> Result<()> {
        self.transport.request("POST", path, None).await?;
        Ok(())
    }

    /// send a PUT request with a JSON body to the given ARI path
    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let json = serde_json::to_string(body).map_err(AriError::Json)?;
        let resp = self.transport.request("PUT", path, Some(json)).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a PUT request with no body to the given ARI path
    pub async fn put_empty(&self, path: &str) -> Result<()> {
        self.transport.request("PUT", path, None).await?;
        Ok(())
    }

    /// send a DELETE request to the given ARI path
    pub async fn delete(&self, path: &str) -> Result<()> {
        self.transport.request("DELETE", path, None).await?;
        Ok(())
    }

    /// send a DELETE request and deserialize the response body
    pub async fn delete_with_response<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.transport.request("DELETE", path, None).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
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

    /// access the underlying config
    pub fn config(&self) -> &AriConfig {
        &self.config
    }

    /// create a pending channel with a pre-generated ID for race-free origination
    ///
    /// the returned PendingChannel subscribes to events for its ID immediately,
    /// so no StasisStart events are missed between originate and subscribe.
    pub fn channel(&self) -> crate::pending::PendingChannel {
        crate::pending::PendingChannel::new(self.clone())
    }

    /// create a pending bridge with a pre-generated ID
    pub fn bridge(&self) -> crate::pending::PendingBridge {
        crate::pending::PendingBridge::new(self.clone())
    }

    /// create a pending playback with a pre-generated ID
    pub fn playback(&self) -> crate::pending::PendingPlayback {
        crate::pending::PendingPlayback::new(self)
    }

    /// shut down the websocket listener and transport
    pub fn disconnect(&self) {
        self.transport.shutdown();
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
