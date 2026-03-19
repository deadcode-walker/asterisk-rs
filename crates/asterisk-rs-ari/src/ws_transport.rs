//! WebSocket transport for unified REST + events over a single connection.
//!
//! when enabled, all REST API calls go through the same WebSocket
//! that carries events. this eliminates the need for a separate HTTP
//! connection and reduces latency for high-throughput applications.
//!
//! requires Asterisk 20.14.0+ / 21.9.0+ / 22.4.0+

use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};

use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventBus;

use crate::error::{AriError, Result};
use crate::event::{AriEvent, AriMessage};
use crate::transport::TransportResponse;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> String {
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("wsreq-{id}")
}

/// REST request envelope sent over websocket
#[derive(serde::Serialize)]
struct WsRestRequest {
    #[serde(rename = "type")]
    type_field: &'static str,
    request_id: String,
    method: String,
    uri: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    content_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    message_body: Option<String>,
}

/// internal command sent from request() to the background task
struct RestCommand {
    request_id: String,
    method: String,
    uri: String,
    content_type: Option<String>,
    message_body: Option<String>,
    response_tx: oneshot::Sender<TransportResponse>,
}

/// default timeout for REST-over-WS requests
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

/// websocket transport — sends REST requests and receives both
/// REST responses and events over a single websocket connection
pub(crate) struct WsTransport {
    command_tx: mpsc::Sender<RestCommand>,
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl WsTransport {
    /// spawn the background websocket task
    pub fn spawn(
        ws_url: String,
        event_bus: EventBus<AriMessage>,
        reconnect: ReconnectPolicy,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (command_tx, command_rx) = mpsc::channel(64);

        let task_handle = tokio::spawn(ws_loop(
            ws_url,
            event_bus,
            reconnect,
            command_rx,
            shutdown_rx,
        ));

        Self {
            command_tx,
            shutdown_tx,
            task_handle,
        }
    }

    /// send a REST request over the websocket and wait for the response
    pub async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<TransportResponse> {
        let request_id = next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        let cmd = RestCommand {
            request_id,
            method: method.to_owned(),
            uri: path.trim_start_matches('/').to_owned(),
            content_type: body.as_ref().map(|_| "application/json".to_owned()),
            message_body: body,
            response_tx,
        };

        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| AriError::Disconnected)?;

        tokio::time::timeout(REQUEST_TIMEOUT, response_rx)
            .await
            .map_err(|_| AriError::WebSocket("REST request timed out".to_owned()))?
            .map_err(|_| AriError::Disconnected)
    }

    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task_handle.abort();
    }
}

impl Drop for WsTransport {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// main websocket loop with reconnection logic
async fn ws_loop(
    ws_url: String,
    event_bus: EventBus<AriMessage>,
    reconnect: ReconnectPolicy,
    mut command_rx: mpsc::Receiver<RestCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut attempt: u32 = 0;

    loop {
        if *shutdown_rx.borrow() {
            tracing::debug!("ws transport shutting down");
            return;
        }

        tracing::info!(url = %ws_url, attempt, "connecting to ARI websocket (unified mode)");

        match tokio_tungstenite::connect_async(&ws_url).await {
            Ok((ws_stream, _response)) => {
                tracing::info!("ARI websocket connected (unified mode)");
                attempt = 0;

                if let Err(should_exit) =
                    handle_connection(ws_stream, &event_bus, &mut command_rx, &mut shutdown_rx)
                        .await
                {
                    if should_exit {
                        return;
                    }
                }

                tracing::warn!("ARI websocket disconnected (unified mode)");
            }
            Err(e) => {
                tracing::warn!(error = %e, attempt, "ARI websocket connection failed");
            }
        }

        if reconnect.max_retries.is_some_and(|max| attempt >= max) {
            tracing::error!(
                attempt,
                "max reconnection attempts reached, stopping ws transport"
            );
            return;
        }

        let delay = reconnect.delay_for_attempt(attempt);
        if delay > Duration::ZERO {
            tracing::info!(?delay, attempt, "waiting before reconnection");
            tokio::select! {
                _ = tokio::time::sleep(delay) => {}
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::debug!("ws transport shutting down during backoff");
                        return;
                    }
                }
            }
        }

        attempt = attempt.saturating_add(1);
    }
}

/// handle a single active websocket connection — multiplexes REST
/// request/response correlation with event delivery
async fn handle_connection(
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    event_bus: &EventBus<AriMessage>,
    command_rx: &mut mpsc::Receiver<RestCommand>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> std::result::Result<(), bool> {
    use tokio_tungstenite::tungstenite::Message;

    let (mut write, mut read) = ws_stream.split();
    let mut pending: HashMap<String, oneshot::Sender<TransportResponse>> = HashMap::new();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        route_text_message(&text, event_bus, &mut pending);
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("received websocket close frame");
                        pending.clear();
                        return Err(false);
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "websocket read error");
                        pending.clear();
                        return Err(false);
                    }
                    None => {
                        pending.clear();
                        return Err(false);
                    }
                    // ping/pong handled by tungstenite, binary frames ignored
                    _ => {}
                }
            }
            cmd = command_rx.recv() => {
                match cmd {
                    Some(cmd) => {
                        let req = WsRestRequest {
                            type_field: "RESTRequest",
                            request_id: cmd.request_id.clone(),
                            method: cmd.method,
                            uri: cmd.uri,
                            content_type: cmd.content_type,
                            message_body: cmd.message_body,
                        };
                        let json = match serde_json::to_string(&req) {
                            Ok(j) => j,
                            Err(e) => {
                                tracing::warn!(error = %e, "failed to serialize REST request");
                                continue;
                            }
                        };
                        pending.insert(cmd.request_id, cmd.response_tx);
                        if let Err(e) = write.send(Message::Text(json.into())).await {
                            tracing::warn!(error = %e, "failed to send REST request");
                            pending.clear();
                            return Err(false);
                        }
                    }
                    None => {
                        // command channel closed — client dropped
                        pending.clear();
                        return Err(true);
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    pending.clear();
                    return Err(true);
                }
            }
        }
    }
}

/// route an incoming text websocket message — REST responses go to
/// pending callers, everything else is published as an ARI event
fn route_text_message(
    text: &str,
    event_bus: &EventBus<AriMessage>,
    pending: &mut HashMap<String, oneshot::Sender<TransportResponse>>,
) {
    match serde_json::from_str::<AriMessage>(text) {
        Ok(msg) => {
            if let AriEvent::RESTResponse {
                ref request_id,
                status_code,
                ref message_body,
                ..
            } = msg.event
            {
                if let Some(tx) = pending.remove(request_id) {
                    let _ = tx.send(TransportResponse {
                        status: status_code as u16,
                        body: message_body.clone(),
                    });
                }
            } else {
                tracing::debug!(?msg, "received ARI event");
                event_bus.publish(msg);
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                payload = %text,
                "failed to deserialize ARI message"
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_ws_rest_request_serialization() {
        let req = WsRestRequest {
            type_field: "RESTRequest",
            request_id: "req-1".into(),
            method: "POST".into(),
            uri: "channels/test-123/answer".into(),
            content_type: None,
            message_body: None,
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&req).expect("serialize"))
                .expect("parse json");

        assert_eq!(json["type"], "RESTRequest");
        assert_eq!(json["request_id"], "req-1");
        assert_eq!(json["method"], "POST");
        assert_eq!(json["uri"], "channels/test-123/answer");
        // optional fields should be absent
        assert!(json.get("content_type").is_none());
        assert!(json.get("message_body").is_none());
    }

    #[test]
    fn test_ws_rest_request_with_body() {
        let req = WsRestRequest {
            type_field: "RESTRequest",
            request_id: "req-2".into(),
            method: "POST".into(),
            uri: "channels/create".into(),
            content_type: Some("application/json".into()),
            message_body: Some(r#"{"endpoint":"PJSIP/100"}"#.into()),
        };
        let json: serde_json::Value =
            serde_json::from_str(&serde_json::to_string(&req).expect("serialize"))
                .expect("parse json");

        assert_eq!(json["content_type"], "application/json");
        assert_eq!(json["message_body"], r#"{"endpoint":"PJSIP/100"}"#);
    }

    #[test]
    fn test_next_request_id_unique() {
        let id1 = next_request_id();
        let id2 = next_request_id();
        assert_ne!(id1, id2);
        assert!(id1.starts_with("wsreq-"));
    }

    #[test]
    fn test_route_rest_response() {
        let event_bus = EventBus::<AriMessage>::new(16);
        let mut pending = HashMap::new();

        let (tx, mut rx) = oneshot::channel();
        pending.insert("req-42".to_owned(), tx);

        let json = r#"{
            "type": "RESTResponse",
            "request_id": "req-42",
            "transaction_id": "",
            "status_code": 204,
            "reason_phrase": "No Content",
            "uri": "channels/test/answer",
            "timestamp": "2025-01-01T00:00:00",
            "application": "test"
        }"#;

        route_text_message(json, &event_bus, &mut pending);

        // pending should be consumed
        assert!(pending.is_empty());

        // receiver should have the response
        let resp = rx.try_recv().expect("should have response");
        assert_eq!(resp.status, 204);
        assert!(resp.body.is_none());
    }

    #[tokio::test]
    async fn test_route_event_to_bus() {
        let event_bus = EventBus::<AriMessage>::new(16);
        let mut sub = event_bus.subscribe();
        let mut pending = HashMap::new();

        let json = r#"{
            "type": "StasisStart",
            "application": "myapp",
            "timestamp": "2025-01-01T00:00:00",
            "channel": {"id": "ch-1", "name": "PJSIP/100", "state": "Ring",
                        "caller": {"name": "", "number": ""},
                        "connected": {"name": "", "number": ""},
                        "dialplan": {"context": "default", "exten": "100", "priority": 1}},
            "args": []
        }"#;

        route_text_message(json, &event_bus, &mut pending);

        let msg = sub.recv().await.expect("should receive event");
        assert_eq!(msg.application, "myapp");
        assert!(matches!(msg.event, AriEvent::StasisStart { .. }));
    }
}
