//! ARI outbound WebSocket server — accepts incoming connections from Asterisk.
//!
//! when Asterisk is configured with outbound websockets, it connects TO
//! your application. this module provides a TCP/WS server that accepts
//! those connections and creates per-connection ARI sessions.
//!
//! requires Asterisk 22+

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use serde::Serialize;
use serde::de::DeserializeOwned;
use tokio::net::TcpListener;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::task::JoinSet;
use tokio::time::Instant;
use tokio_tungstenite::tungstenite::Message;

use asterisk_rs_core::event::{EventBus, EventSubscription, FilteredSubscription};

use crate::config::{
    DEFAULT_MAX_RESPONSE_BODY_BYTES, DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES, DEFAULT_REQUEST_TIMEOUT,
};
use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::transport::{
    PendingResponse, REST_COMMAND_CAPACITY, RequestLifecycle, RestCommand, TransportResponse,
    deadline_error, fail_pending, outbound_message_limit_error, poll_wire_write, purge_expired,
    route_text_message, write_error,
};
use crate::websocket::websocket_config;
use crate::ws_proto::WsRestRequest;

/// per-session request id counter — only needs uniqueness within a session,
/// but a global counter keeps ids distinct across sessions for tracing
static SESSION_REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> String {
    // relaxed is sufficient: fetch_add is an atomic RMW — it cannot return
    // the same value to two threads. no other memory operations need
    // ordering relative to this counter
    let id = SESSION_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("srv-{id}")
}

const CANCELLED_REQUEST_CLEANUP_INTERVAL: Duration = Duration::from_secs(1);
const DEFAULT_MAX_CONNECTIONS: usize = 256;
const DEFAULT_HANDSHAKE_TIMEOUT: Duration = Duration::from_secs(10);
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);
const CLOSE_TIMEOUT: Duration = Duration::from_secs(1);

#[derive(Clone, Copy)]
struct ConnectionConfig {
    handshake_timeout: Duration,
    shutdown_timeout: Duration,
    request_timeout: Duration,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
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
/// If the remote Asterisk disconnects after a mutating request is written but
/// before its response arrives, that request fails with
/// [`AriError::OutcomeUnknown`].
#[derive(Debug, Clone)]
pub struct AriSession {
    event_bus: EventBus<AriMessage>,
    command_tx: mpsc::Sender<RestCommand>,
    shutdown_tx: Arc<watch::Sender<bool>>,
    shutdown_rx: watch::Receiver<bool>,
    peer_addr: SocketAddr,
    request_timeout: Duration,
}

impl AriSession {
    fn new(
        peer_addr: SocketAddr,
        request_timeout: Duration,
    ) -> (Self, mpsc::Receiver<RestCommand>, watch::Receiver<bool>) {
        let event_bus = EventBus::new(256);
        let (command_tx, command_rx) = mpsc::channel(REST_COMMAND_CAPACITY);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        (
            Self {
                event_bus,
                command_tx,
                shutdown_tx: Arc::new(shutdown_tx),
                shutdown_rx: shutdown_rx.clone(),
                peer_addr,
                request_timeout,
            },
            command_rx,
            shutdown_rx,
        )
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

    /// wait until this session or its owning server starts shutting down
    pub async fn cancelled(&self) {
        let mut shutdown_rx = self.shutdown_rx.clone();
        wait_for_shutdown(&mut shutdown_rx).await;
    }

    /// whether this session has started shutting down
    pub fn is_cancelled(&self) -> bool {
        *self.shutdown_rx.borrow()
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
    ) -> Result<TransportResponse> {
        let deadline = Instant::now()
            .checked_add(self.request_timeout)
            .ok_or_else(|| {
                AriError::InvalidConfig(
                    "request_timeout is too large for the platform clock".to_owned(),
                )
            })?;
        let request_id = next_request_id();
        let (response_tx, mut response_rx) = oneshot::channel();
        let lifecycle = Arc::new(RequestLifecycle::default());
        let uri = path.strip_prefix('/').unwrap_or(path).to_owned();

        let cmd = RestCommand {
            request_id: request_id.clone(),
            method: method.to_owned(),
            uri: uri.clone(),
            content_type: body.as_ref().map(|_| "application/json".to_owned()),
            message_body: body,
            deadline,
            lifecycle: lifecycle.clone(),
            response_tx,
        };

        match tokio::time::timeout_at(deadline, async {
            self.command_tx
                .send(cmd)
                .await
                .map_err(|_| AriError::Disconnected)?;

            match (&mut response_rx).await {
                Ok(result) => result,
                Err(_) => Err(write_error(method, &uri, &request_id, &lifecycle, || {
                    "session stopped before returning a REST response".to_owned()
                })),
            }
        })
        .await
        {
            Ok(result) => result,
            Err(_) => {
                if let Ok(result) = response_rx.try_recv() {
                    return result;
                }
                Err(deadline_error(method, &uri, &request_id, &lifecycle))
            }
        }
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
    max_connections: usize,
    handshake_timeout: Duration,
    shutdown_timeout: Duration,
    request_timeout: Duration,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
    admission_hook: Arc<dyn Fn(SocketAddr) -> bool + Send + Sync>,
}

impl std::fmt::Debug for AriServer {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriServer")
            .field("local_addr", &self.listener.local_addr().ok())
            .field("max_connections", &self.max_connections)
            .field("handshake_timeout", &self.handshake_timeout)
            .field("shutdown_timeout", &self.shutdown_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("max_response_body_bytes", &self.max_response_body_bytes)
            .field(
                "max_websocket_message_bytes",
                &self.max_websocket_message_bytes,
            )
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
        let mut tasks = JoinSet::new();

        loop {
            if tasks.len() >= self.max_connections {
                tokio::select! {
                    biased;
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            break;
                        }
                    }
                    result = tasks.join_next() => {
                        report_connection_result(result)?;
                    }
                }
                continue;
            }

            tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        break;
                    }
                }
                result = tasks.join_next(), if !tasks.is_empty() => {
                    report_connection_result(result)?;
                }
                result = self.listener.accept() => {
                    let (stream, addr) = match result {
                        Ok(v) => v,
                        Err(e) => {
                            tracing::warn!(error = %e, "accept error, retrying in 100ms");
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };
                    if !(self.admission_hook)(addr) {
                        tracing::warn!(%addr, "rejected incoming ARI websocket connection");
                        continue;
                    }
                    tracing::info!(%addr, "accepted incoming ARI websocket connection");

                    let handler = handler.clone();
                    let connection_shutdown_rx = shutdown_rx.clone();
                    let connection_config = ConnectionConfig {
                        handshake_timeout: self.handshake_timeout,
                        shutdown_timeout: self.shutdown_timeout,
                        request_timeout: self.request_timeout,
                        max_response_body_bytes: self.max_response_body_bytes,
                        max_websocket_message_bytes: self.max_websocket_message_bytes,
                    };
                    tasks.spawn(async move {
                        run_connection(
                            stream,
                            addr,
                            handler,
                            connection_shutdown_rx,
                            connection_config,
                        )
                        .await;
                    });
                }
            }
        }

        tracing::info!(active_connections = tasks.len(), "ARI server shutting down");
        drain_connections(&mut tasks, self.shutdown_timeout).await;
        Ok(())
    }
}

async fn run_connection<F, Fut>(
    stream: tokio::net::TcpStream,
    addr: SocketAddr,
    handler: Arc<F>,
    mut server_shutdown_rx: watch::Receiver<bool>,
    config: ConnectionConfig,
) where
    F: Fn(AriSession) -> Fut + Send + Sync + 'static,
    Fut: std::future::Future<Output = ()> + Send + 'static,
{
    let handshake = tokio::time::timeout(
        config.handshake_timeout,
        tokio_tungstenite::accept_async_with_config(
            stream,
            Some(websocket_config(config.max_websocket_message_bytes)),
        ),
    );
    tokio::pin!(handshake);

    let ws_stream = tokio::select! {
        biased;
        _ = wait_for_shutdown(&mut server_shutdown_rx) => return,
        result = &mut handshake => {
            match result {
                Ok(Ok(ws_stream)) => ws_stream,
                Ok(Err(error)) => {
                    tracing::warn!(error = %error, %addr, "websocket handshake failed");
                    return;
                }
                Err(_) => {
                    tracing::warn!(%addr, handshake_timeout = ?config.handshake_timeout, "websocket handshake timed out");
                    return;
                }
            }
        }
    };

    let (session, command_rx, session_shutdown_rx) = AriSession::new(addr, config.request_timeout);
    let session_shutdown_tx = Arc::clone(&session.shutdown_tx);
    let event_bus = session.event_bus.clone();
    // closures that do not use their session argument may drop it before their
    // returned future is first polled; retain one handle for the task lifetime
    let session_lifetime = session.clone();
    let session_task = session_loop(
        ws_stream,
        event_bus,
        command_rx,
        session_shutdown_rx,
        config.max_response_body_bytes,
        config.max_websocket_message_bytes,
    );
    let handler_task = handler(session);
    tokio::pin!(session_task);
    tokio::pin!(handler_task);

    enum Completion {
        ServerShutdown,
        Session,
        Handler,
    }

    let completion = tokio::select! {
        biased;
        _ = wait_for_shutdown(&mut server_shutdown_rx) => Completion::ServerShutdown,
        _ = &mut session_task => Completion::Session,
        _ = &mut handler_task => Completion::Handler,
    };

    let _ = session_shutdown_tx.send(true);
    let drained = match completion {
        Completion::ServerShutdown => tokio::time::timeout(config.shutdown_timeout, async {
            tokio::join!(&mut session_task, &mut handler_task);
        })
        .await
        .is_ok(),
        Completion::Session => tokio::time::timeout(config.shutdown_timeout, &mut handler_task)
            .await
            .is_ok(),
        Completion::Handler => tokio::time::timeout(config.shutdown_timeout, &mut session_task)
            .await
            .is_ok(),
    };
    if !drained {
        tracing::warn!(%addr, shutdown_timeout = ?config.shutdown_timeout, "ARI connection tasks did not drain before timeout");
    }
    drop(session_lifetime);
}

async fn wait_for_shutdown(shutdown_rx: &mut watch::Receiver<bool>) {
    loop {
        if *shutdown_rx.borrow() {
            return;
        }
        if shutdown_rx.changed().await.is_err() {
            return;
        }
    }
}

fn report_connection_result(
    result: Option<std::result::Result<(), tokio::task::JoinError>>,
) -> Result<()> {
    if let Some(Err(error)) = result {
        tracing::error!(error = %error, "ARI connection task failed");
        if error.is_panic() {
            return Err(AriError::SessionTaskFailed {
                details: format!("ARI connection task panicked: {error}"),
            });
        }
    }
    Ok(())
}

async fn drain_connections(tasks: &mut JoinSet<()>, timeout: Duration) {
    let drain = async {
        while let Some(result) = tasks.join_next().await {
            let _ = report_connection_result(Some(result));
        }
    };

    if tokio::time::timeout(timeout, drain).await.is_err() {
        tracing::warn!(
            active_connections = tasks.len(),
            ?timeout,
            "aborting ARI connection tasks after shutdown timeout"
        );
        tasks.abort_all();
        while let Some(result) = tasks.join_next().await {
            let _ = report_connection_result(Some(result));
        }
    }
}

// --- AriServerBuilder ---

/// builder for [`AriServer`]
#[derive(Clone)]
#[must_use]
pub struct AriServerBuilder {
    bind_addr: SocketAddr,
    max_connections: usize,
    handshake_timeout: Duration,
    shutdown_timeout: Duration,
    request_timeout: Duration,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
    allow_external_bind: bool,
    admission_hook: Arc<dyn Fn(SocketAddr) -> bool + Send + Sync>,
}

impl std::fmt::Debug for AriServerBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriServerBuilder")
            .field("bind_addr", &self.bind_addr)
            .field("max_connections", &self.max_connections)
            .field("handshake_timeout", &self.handshake_timeout)
            .field("shutdown_timeout", &self.shutdown_timeout)
            .field("request_timeout", &self.request_timeout)
            .field("max_response_body_bytes", &self.max_response_body_bytes)
            .field(
                "max_websocket_message_bytes",
                &self.max_websocket_message_bytes,
            )
            .field("allow_external_bind", &self.allow_external_bind)
            .finish_non_exhaustive()
    }
}

impl AriServerBuilder {
    /// create a builder with default bind address `127.0.0.1:8765`
    pub fn new() -> Self {
        Self {
            bind_addr: ([127, 0, 0, 1], 8765).into(),
            max_connections: DEFAULT_MAX_CONNECTIONS,
            handshake_timeout: DEFAULT_HANDSHAKE_TIMEOUT,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_response_body_bytes: DEFAULT_MAX_RESPONSE_BODY_BYTES,
            max_websocket_message_bytes: DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES,
            allow_external_bind: false,
            admission_hook: Arc::new(|_| true),
        }
    }

    /// set the address to listen on (default `127.0.0.1:8765`)
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// Explicitly permit an external bind after the caller supplies a network auth boundary.
    pub fn allow_external_bind(mut self, allow: bool) -> Self {
        self.allow_external_bind = allow;
        self
    }

    /// Admit or reject a peer before spending work on its WebSocket handshake.
    pub fn admission_hook(
        mut self,
        hook: impl Fn(SocketAddr) -> bool + Send + Sync + 'static,
    ) -> Self {
        self.admission_hook = Arc::new(hook);
        self
    }

    /// limit concurrent handshakes and active sessions (default 256)
    pub fn max_connections(mut self, max: usize) -> Self {
        self.max_connections = max;
        self
    }

    /// limit the websocket upgrade handshake (default 10 seconds)
    pub fn handshake_timeout(mut self, timeout: Duration) -> Self {
        self.handshake_timeout = timeout;
        self
    }

    /// limit graceful task draining during server shutdown (default 5 seconds)
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// set the deadline for one session REST operation (default 30 seconds)
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// cap REST response bodies received over a session (default 4 MiB)
    pub fn max_response_body_bytes(mut self, bytes: usize) -> Self {
        self.max_response_body_bytes = bytes;
        self
    }

    /// cap inbound messages/frames and outbound REST envelopes (default 4 MiB)
    pub fn max_websocket_message_bytes(mut self, bytes: usize) -> Self {
        self.max_websocket_message_bytes = bytes;
        self
    }

    /// bind the TCP listener and return the server + shutdown handle
    pub async fn build(self) -> Result<(AriServer, ShutdownHandle)> {
        if !self.bind_addr.ip().is_loopback() && !self.allow_external_bind {
            return Err(AriError::InvalidConfig(
                "external bind requires allow_external_bind(true) and a network authentication boundary"
                    .to_owned(),
            ));
        }
        if self.max_connections == 0 {
            return Err(AriError::InvalidConfig(
                "max_connections must be greater than zero".to_owned(),
            ));
        }
        if self.handshake_timeout.is_zero() {
            return Err(AriError::InvalidConfig(
                "handshake_timeout must be greater than zero".to_owned(),
            ));
        }
        if self.shutdown_timeout.is_zero() {
            return Err(AriError::InvalidConfig(
                "shutdown_timeout must be greater than zero".to_owned(),
            ));
        }
        if self.request_timeout.is_zero() {
            return Err(AriError::InvalidConfig(
                "request_timeout must be greater than zero".to_owned(),
            ));
        }
        if std::time::Instant::now()
            .checked_add(self.request_timeout)
            .is_none()
        {
            return Err(AriError::InvalidConfig(
                "request_timeout is too large for the platform clock".to_owned(),
            ));
        }
        if self.max_response_body_bytes == 0 {
            return Err(AriError::InvalidConfig(
                "max_response_body_bytes must be greater than zero".to_owned(),
            ));
        }
        if self.max_websocket_message_bytes == 0 {
            return Err(AriError::InvalidConfig(
                "max_websocket_message_bytes must be greater than zero".to_owned(),
            ));
        }

        let listener = TcpListener::bind(self.bind_addr)
            .await
            .map_err(AriError::Io)?;

        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let server = AriServer {
            listener,
            shutdown_rx,
            max_connections: self.max_connections,
            handshake_timeout: self.handshake_timeout,
            shutdown_timeout: self.shutdown_timeout,
            request_timeout: self.request_timeout,
            max_response_body_bytes: self.max_response_body_bytes,
            max_websocket_message_bytes: self.max_websocket_message_bytes,
            admission_hook: self.admission_hook,
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
    mut command_rx: mpsc::Receiver<RestCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
) {
    let (mut write, mut read) = ws_stream.split();
    let mut pending: HashMap<String, PendingResponse> = HashMap::new();

    loop {
        purge_expired(&mut pending);
        let next_deadline = pending
            .values()
            .map(|response| response.deadline)
            .min()
            .map(|deadline| deadline.min(Instant::now() + CANCELLED_REQUEST_CLEANUP_INTERVAL));
        let pending_timeout = async move {
            match next_deadline {
                Some(deadline) => tokio::time::sleep_until(deadline).await,
                None => std::future::pending().await,
            }
        };
        tokio::pin!(pending_timeout);

        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    tracing::debug!("session shutting down");
                    fail_pending(&mut pending, "session shut down before a response arrived");
                    match tokio::time::timeout(CLOSE_TIMEOUT, write.send(Message::Close(None))).await {
                        Ok(Ok(())) => {}
                        Ok(Err(error)) => tracing::debug!(error = %error, "failed to send session close frame"),
                        Err(_) => tracing::debug!("timed out sending session close frame"),
                    }
                    return;
                }
            }
            _ = &mut pending_timeout => {
                purge_expired(&mut pending);
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Text(text))) => {
                        route_text_message(
                            &text,
                            &event_bus,
                            &mut pending,
                            max_response_body_bytes,
                        );
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("session received websocket close frame");
                        fail_pending(&mut pending, "session closed before a response arrived");
                        return;
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "session websocket read error");
                        fail_pending(&mut pending, "session read failed before a response arrived");
                        return;
                    }
                    None => {
                        fail_pending(&mut pending, "session ended before a response arrived");
                        return;
                    },
                    // ping/pong handled by tungstenite, binary frames ignored
                    _ => {}
                }
            }
            cmd = command_rx.recv(), if pending.len() < REST_COMMAND_CAPACITY => {
                match cmd {
                    Some(cmd) => {
                        // timed-out requests must not execute after the session catches up
                        if cmd.response_tx.is_closed() {
                            cmd.lifecycle.cancel_unsent();
                            tracing::debug!(
                                request_id = %cmd.request_id,
                                "discarding expired session request"
                            );
                            continue;
                        }
                        if cmd.deadline <= Instant::now() {
                            let error = deadline_error(
                                &cmd.method,
                                &cmd.uri,
                                &cmd.request_id,
                                &cmd.lifecycle,
                            );
                            let _ = cmd.response_tx.send(Err(error));
                            continue;
                        }
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
                        if let Some(error) = outbound_message_limit_error(
                            &req.method,
                            &req.uri,
                            json.len(),
                            max_websocket_message_bytes,
                        ) {
                            tracing::warn!(
                                request_id = %cmd.request_id,
                                message_bytes = json.len(),
                                limit = max_websocket_message_bytes,
                                "session REST request exceeds websocket message limit"
                            );
                            let _ = cmd.response_tx.send(Err(error));
                            continue;
                        }
                        if cmd.response_tx.is_closed() {
                            cmd.lifecycle.cancel_unsent();
                            continue;
                        }
                        if cmd.deadline <= Instant::now() {
                            let error = deadline_error(
                                &req.method,
                                &req.uri,
                                &cmd.request_id,
                                &cmd.lifecycle,
                            );
                            let _ = cmd.response_tx.send(Err(error));
                            continue;
                        }

                        let write_result = tokio::select! {
                            biased;
                            _ = wait_for_shutdown(&mut shutdown_rx) => {
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || "session shut down during REST request write".to_owned(),
                                );
                                let _ = cmd.response_tx.send(Err(result));
                                fail_pending(&mut pending, "session shut down before a response arrived");
                                return;
                            }
                            result = tokio::time::timeout_at(
                                cmd.deadline,
                                poll_wire_write(
                                    &cmd.lifecycle,
                                    write.send(Message::Text(json.into())),
                                ),
                            ) => result,
                        };

                        match write_result {
                            Ok(Some(Ok(()))) => {
                                cmd.lifecycle.mark_written();
                                pending.insert(
                                    cmd.request_id.clone(),
                                    PendingResponse {
                                        deadline: cmd.deadline,
                                        request_id: cmd.request_id,
                                        method: req.method,
                                        uri: req.uri,
                                        lifecycle: cmd.lifecycle,
                                        response_tx: cmd.response_tx,
                                    },
                                );
                            }
                            Ok(Some(Err(error))) => {
                                tracing::warn!(error = %error, "failed to send REST request");
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || format!("failed to send REST request: {error}"),
                                );
                                let _ = cmd.response_tx.send(Err(result));
                                fail_pending(
                                    &mut pending,
                                    "session write failed before a response arrived",
                                );
                                return;
                            }
                            Ok(None) => {
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || "session request was cancelled before its first wire poll".to_owned(),
                                );
                                let _ = cmd.response_tx.send(Err(result));
                            }
                            Err(_) => {
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || format!("{} {} wire write timed out", req.method, req.uri),
                                );
                                let _ = cmd.response_tx.send(Err(result));
                                fail_pending(
                                    &mut pending,
                                    "session write timed out before a response arrived",
                                );
                                return;
                            }
                        }
                    }
                    // all session handles dropped
                    None => {
                        fail_pending(&mut pending, "session request channel closed");
                        let _ = tokio::time::timeout(
                            CLOSE_TIMEOUT,
                            write.send(Message::Close(None)),
                        )
                        .await;
                        return;
                    }
                }
            }
        }
    }
}
