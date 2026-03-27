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
use crate::util::redact_url;
use crate::ws_proto::WsRestRequest;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> String {
    // relaxed is sufficient: fetch_add is an atomic RMW — it cannot return
    // the same value to two threads. no other memory operations need
    // ordering relative to this counter
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("wsreq-{id}")
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
            uri: path.strip_prefix('/').unwrap_or(path).to_owned(),
            content_type: body.as_ref().map(|_| "application/json".to_owned()),
            message_body: body,
            response_tx,
        };

        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| AriError::Disconnected)?;

        let response = tokio::time::timeout(REQUEST_TIMEOUT, response_rx)
            .await
            .map_err(|_| AriError::WebSocket("REST request timed out".to_owned()))?
            .map_err(|_| AriError::Disconnected)?;

        if response.status >= 400 {
            let message = response.body.unwrap_or_else(|| "request failed".to_owned());
            return Err(AriError::Api {
                status: response.status,
                message,
            });
        }
        Ok(response)
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

        tracing::info!(url = %redact_url(&ws_url), attempt, "connecting to ARI websocket (unified mode)");

        match tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async(&ws_url),
        )
        .await
        {
            Err(_) => {
                tracing::warn!(attempt, "ARI websocket connection timed out");
            }
            Ok(Ok((ws_stream, _response))) => {
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
            Ok(Err(e)) => {
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
        // purge entries whose receiver was dropped (e.g. caller-side timeout)
        pending.retain(|_, tx| !tx.is_closed());
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
            tracing::warn!(error = %e, "failed to deserialize ARI message");
            tracing::trace!(payload = %text, "raw ARI message payload");
        }
    }
}
