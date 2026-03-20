//! ARI outbound WebSocket server — accepts incoming connections from Asterisk.
//!
//! when Asterisk is configured with outbound websockets, it connects TO
//! your application. this module provides a TCP/WS server that accepts
//! those connections and creates per-connection ARI sessions.
//!
//! requires Asterisk 22+

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::de::DeserializeOwned;
use serde::Serialize;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio_tungstenite::tungstenite::Message;

use asterisk_rs_core::event::{EventBus, EventSubscription, FilteredSubscription};

use crate::error::{AriError, Result};
use crate::event::{AriEvent, AriMessage};

/// per-session request id counter — only needs uniqueness within a session,
/// but a global counter keeps ids distinct across sessions for tracing
static SESSION_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> String {
    let id = SESSION_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("srv-{id}")
}

/// default timeout for REST-over-WS requests
const REQUEST_TIMEOUT: Duration = Duration::from_secs(30);

// --- REST-over-WS protocol types (duplicated from ws_transport to keep modules decoupled) ---

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

/// internal command sent from request methods to the session background task
struct SessionCommand {
    request_id: String,
    method: String,
    uri: String,
    content_type: Option<String>,
    message_body: Option<String>,
    response_tx: oneshot::Sender<SessionResponse>,
}

/// response from a REST-over-WS request within a session
struct SessionResponse {
    status: u16,
    body: Option<String>,
}

// --- public types ---

/// handle for controlling the ARI server lifecycle
#[derive(Debug, Clone)]
pub struct ShutdownHandle {
    shutdown_tx: Arc<watch::Sender<bool>>,
}

impl ShutdownHandle {
    /// signal the server to stop accepting new connections
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
    }
}

/// a session representing a single incoming Asterisk WebSocket connection
///
/// provides REST methods and event subscriptions scoped to this connection.
/// when the remote Asterisk disconnects, outstanding requests fail with
/// [`AriError::Disconnected`].
#[derive(Debug, Clone)]
pub struct AriSession {
    event_bus: EventBus<AriMessage>,
    command_tx: mpsc::Sender<SessionCommand>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    peer_addr: SocketAddr,
}

impl AriSession {
    fn from_ws_stream(
        ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
        peer_addr: SocketAddr,
    ) -> Self {
        let event_bus = EventBus::new(256);
        let (command_tx, command_rx) = mpsc::channel(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let bus = event_bus.clone();
        tokio::spawn(session_loop(ws_stream, bus, command_rx, shutdown_rx));

        Self {
            event_bus,
            command_tx,
            shutdown_tx: Arc::new(shutdown_tx),
            peer_addr,
        }
    }

    /// subscribe to all ARI events on this connection
    pub fn subscribe(&self) -> EventSubscription<AriMessage> {
        self.event_bus.subscribe()
    }

    /// subscribe to events matching a filter predicate
    pub fn subscribe_filtered(
        &self,
        predicate: impl Fn(&AriMessage) -> bool + Send + 'static,
    ) -> FilteredSubscription<AriMessage> {
        self.event_bus.subscribe_filtered(predicate)
    }

    /// access the underlying event bus for this session
    pub fn events(&self) -> &EventBus<AriMessage> {
        &self.event_bus
    }

    /// remote address of the connected Asterisk instance
    pub fn peer_addr(&self) -> SocketAddr {
        self.peer_addr
    }

    /// close this session's websocket connection
    pub fn disconnect(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// send a GET request over this session's websocket
    pub async fn get<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.raw_request("GET", path, None).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a POST request with a JSON body over this session's websocket
    pub async fn post<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let json = serde_json::to_string(body).map_err(AriError::Json)?;
        let resp = self.raw_request("POST", path, Some(json)).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a POST request with no body
    pub async fn post_empty(&self, path: &str) -> Result<()> {
        self.raw_request("POST", path, None).await?;
        Ok(())
    }

    /// send a PUT request with a JSON body over this session's websocket
    pub async fn put<T: DeserializeOwned>(&self, path: &str, body: &impl Serialize) -> Result<T> {
        let json = serde_json::to_string(body).map_err(AriError::Json)?;
        let resp = self.raw_request("PUT", path, Some(json)).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a PUT request with no body
    pub async fn put_empty(&self, path: &str) -> Result<()> {
        self.raw_request("PUT", path, None).await?;
        Ok(())
    }

    /// send a DELETE request
    pub async fn delete(&self, path: &str) -> Result<()> {
        self.raw_request("DELETE", path, None).await?;
        Ok(())
    }

    /// send a DELETE request and deserialize the response body
    pub async fn delete_with_response<T: DeserializeOwned>(&self, path: &str) -> Result<T> {
        let resp = self.raw_request("DELETE", path, None).await?;
        let body = resp.body.ok_or_else(|| AriError::Api {
            status: resp.status,
            message: "expected response body".into(),
        })?;
        serde_json::from_str(&body).map_err(AriError::Json)
    }

    /// send a raw REST-over-WS request and wait for the correlated response
    async fn raw_request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<SessionResponse> {
        let request_id = next_request_id();
        let (response_tx, response_rx) = oneshot::channel();

        let cmd = SessionCommand {
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

        let resp = tokio::time::timeout(REQUEST_TIMEOUT, response_rx)
            .await
            .map_err(|_| AriError::WebSocket("REST request timed out".to_owned()))?
            .map_err(|_| AriError::Disconnected)?;

        // mirror HttpTransport: any non-2xx status is an error
        if resp.status < 200 || resp.status >= 300 {
            let message = resp.body.unwrap_or_else(|| format!("HTTP {}", resp.status));
            return Err(AriError::Api {
                status: resp.status,
                message,
            });
        }
        Ok(resp)
    }
}

// --- AriServer ---

/// ARI outbound websocket server
///
/// listens for incoming websocket connections from Asterisk instances
/// configured with outbound websockets (Asterisk 22+)
pub struct AriServer {
    listener: TcpListener,
    shutdown_rx: watch::Receiver<bool>,
}

impl std::fmt::Debug for AriServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriServer")
            .field("local_addr", &self.listener.local_addr().ok())
            .finish_non_exhaustive()
    }
}

impl AriServer {
    pub fn builder() -> AriServerBuilder {
        AriServerBuilder::new()
    }

    /// local address the server is bound to
    pub fn local_addr(&self) -> Result<SocketAddr> {
        self.listener.local_addr().map_err(AriError::Io)
    }

    /// accept incoming connections and call the handler for each
    ///
    /// runs until shutdown is signaled or the listener errors.
    /// each accepted connection is upgraded to websocket and handed
    /// to `handler` as an [`AriSession`] on a spawned task.
    pub async fn run<F, Fut>(self, handler: F) -> Result<()>
    where
        F: Fn(AriSession) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send + 'static,
    {
        let handler = Arc::new(handler);
        let mut shutdown_rx = self.shutdown_rx;

        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    let (stream, addr) = match result {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(error = %e, "accept error, retrying in 100ms");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };
                    tracing::info!(%addr, "accepted incoming ARI websocket connection");

                    let handler = handler.clone();
                    tokio::spawn(async move {
                        match tokio_tungstenite::accept_async(stream).await {
                            Ok(ws_stream) => {
                                let session = AriSession::from_ws_stream(ws_stream, addr);
                                handler(session).await;
                            }
                            Err(e) => {
                                tracing::warn!(
                                    error = %e, %addr,
                                    "websocket handshake failed"
                                );
                            }
                        }
                    });
                }
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::info!("ARI server shutting down");
                        return Ok(());
                    }
                }
            }
        }
    }
}

// --- AriServerBuilder ---

/// builder for [`AriServer`]
#[derive(Debug, Clone)]
pub struct AriServerBuilder {
    bind_addr: SocketAddr,
}

impl AriServerBuilder {
    pub fn new() -> Self {
        Self {
            bind_addr: ([0, 0, 0, 0], 8765).into(),
        }
    }

    /// set the address to listen on (default `0.0.0.0:8765`)
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// bind the TCP listener and return the server + shutdown handle
    pub async fn build(self) -> Result<(AriServer, ShutdownHandle)> {
        let listener = TcpListener::bind(self.bind_addr)
            .await
            .map_err(AriError::Io)?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let server = AriServer {
            listener,
            shutdown_rx,
        };
        let handle = ShutdownHandle {
            shutdown_tx: Arc::new(shutdown_tx),
        };

        Ok((server, handle))
    }
}

impl Default for AriServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

// --- session background task ---

/// background loop for a single accepted websocket — routes REST responses
/// to pending callers and publishes events to the session's event bus
async fn session_loop(
    ws_stream: tokio_tungstenite::WebSocketStream<tokio::net::TcpStream>,
    event_bus: EventBus<AriMessage>,
    mut command_rx: mpsc::Receiver<SessionCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let (mut write, mut read) = ws_stream.split();
    let mut pending: HashMap<String, oneshot::Sender<SessionResponse>> = HashMap::new();

    loop {
        tokio::select! {
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        route_message(&text, &event_bus, &mut pending);
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("session received websocket close frame");
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "session websocket read error");
                        return;
                    }
                    None => return,
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
                        if let Err(e) = write.send(Message::Text(json)).await {
                            tracing::warn!(error = %e, "failed to send REST request");
                            return;
                        }
                    }
                    // all session handles dropped
                    None => return,
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    tracing::debug!("session shutting down");
                    return;
                }
            }
        }
    }
}

/// route an incoming text message — REST responses go to pending callers,
/// everything else is published as an ARI event
fn route_message(
    text: &str,
    event_bus: &EventBus<AriMessage>,
    pending: &mut HashMap<String, oneshot::Sender<SessionResponse>>,
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
                    let _ = tx.send(SessionResponse {
                        status: status_code as u16,
                        body: message_body.clone(),
                    });
                }
            } else {
                event_bus.publish(msg);
            }
        }
        Err(e) => {
            tracing::warn!(
                error = %e,
                payload = %text,
                "failed to deserialize ARI message in session"
            );
        }
    }
}
