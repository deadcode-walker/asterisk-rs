//! WebSocket transport for unified REST + events over a single connection.
//!
//! when enabled, all REST API calls go through the same WebSocket
//! that carries events. this eliminates the need for a separate HTTP
//! connection and reduces latency for high-throughput applications.
//!
//! requires Asterisk 20.14.0+ / 21.9.0+ / 22.4.0+

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU64, Ordering};
use std::time::Duration;

use futures_util::{SinkExt, StreamExt};
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;

use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventBus;

use crate::config::AriConfig;
use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::transport::{
    AriConnectionState, PendingResponse, REST_COMMAND_CAPACITY, RequestLifecycle, RestCommand,
    TransportResponse, deadline_error, fail_pending, outbound_message_limit_error, poll_wire_write,
    purge_expired, route_text_message, write_error,
};
use crate::util::redact_url;
use crate::websocket::{OwnedTask, connector_for_url, websocket_config};
use crate::ws_proto::WsRestRequest;

static REQUEST_COUNTER: AtomicU64 = AtomicU64::new(1);

fn next_request_id() -> String {
    // relaxed is sufficient: fetch_add is an atomic RMW — it cannot return
    // the same value to two threads. no other memory operations need
    // ordering relative to this counter
    let id = REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed);
    format!("wsreq-{id}")
}

const CANCELLED_REQUEST_CLEANUP_INTERVAL: Duration = Duration::from_secs(1);

struct WsLoopConfig {
    reconnect: ReconnectPolicy,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
}

/// websocket transport — sends REST requests and receives both
/// REST responses and events over a single websocket connection
pub(crate) struct WsTransport {
    command_tx: mpsc::Sender<RestCommand>,
    shutdown_tx: watch::Sender<bool>,
    state_rx: watch::Receiver<AriConnectionState>,
    task: OwnedTask,
    request_timeout: Duration,
}

impl WsTransport {
    /// spawn the background websocket task
    pub(crate) fn spawn(config: &AriConfig, event_bus: EventBus<AriMessage>) -> Result<Self> {
        let ws_url = config.ws_url().to_string();
        let tls_connector = connector_for_url(&ws_url, &config.tls_trust.rustls_roots)?;
        let loop_config = WsLoopConfig {
            reconnect: config.reconnect_policy().clone(),
            max_response_body_bytes: config.max_response_body_bytes(),
            max_websocket_message_bytes: config.max_websocket_message_bytes(),
        };
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        let (state_tx, state_rx) = watch::channel(AriConnectionState::Connecting);
        let (command_tx, command_rx) = mpsc::channel(REST_COMMAND_CAPACITY);

        let task = tokio::spawn(ws_loop(
            ws_url,
            event_bus,
            tls_connector,
            loop_config,
            command_rx,
            shutdown_rx,
            state_tx,
        ));

        Ok(Self {
            command_tx,
            shutdown_tx,
            state_rx,
            task: OwnedTask::new(task),
            request_timeout: config.request_timeout(),
        })
    }

    pub(crate) fn connection_state(&self) -> AriConnectionState {
        self.state_rx.borrow().clone()
    }

    pub(crate) fn subscribe_connection_state(&self) -> watch::Receiver<AriConnectionState> {
        self.state_rx.clone()
    }

    /// send a REST request over the websocket and wait for the response
    pub(crate) async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<TransportResponse> {
        if self.connection_state() != AriConnectionState::Ready {
            return Err(AriError::RequestNotSent {
                method: method.to_owned(),
                uri: path.strip_prefix('/').unwrap_or(path).to_owned(),
            });
        }
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
                    "websocket transport stopped before returning a REST response".to_owned()
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

    pub(crate) async fn shutdown_and_wait(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task
            .shutdown_and_wait("ARI unified websocket transport")
            .await;
    }

    pub(crate) fn abort(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task.abort();
    }
}

impl Drop for WsTransport {
    fn drop(&mut self) {
        self.abort();
    }
}

/// main websocket loop with reconnection logic
async fn ws_loop(
    ws_url: String,
    event_bus: EventBus<AriMessage>,
    tls_connector: tokio_tungstenite::Connector,
    config: WsLoopConfig,
    mut command_rx: mpsc::Receiver<RestCommand>,
    mut shutdown_rx: watch::Receiver<bool>,
    state_tx: watch::Sender<AriConnectionState>,
) {
    let mut attempt: u32 = 0;

    loop {
        if *shutdown_rx.borrow() {
            state_tx.send_replace(AriConnectionState::Disconnected);
            tracing::debug!("ws transport shutting down");
            return;
        }
        state_tx.send_replace(if attempt == 0 {
            AriConnectionState::Connecting
        } else {
            AriConnectionState::Reconnecting
        });

        tracing::info!(url = %redact_url(&ws_url), attempt, "connecting to ARI websocket (unified mode)");

        let connection = tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async_tls_with_config(
                &ws_url,
                Some(websocket_config(config.max_websocket_message_bytes)),
                false,
                Some(tls_connector.clone()),
            ),
        );
        tokio::pin!(connection);
        let connection_result = loop {
            tokio::select! {
                biased;
                _ = wait_for_shutdown(&mut shutdown_rx) => {
                    state_tx.send_replace(AriConnectionState::Disconnected);
                    tracing::debug!("ws transport shutting down during connection attempt");
                    return;
                }
                command = command_rx.recv() => match command { Some(command) => reject_unready(command), None => return },
                result = &mut connection => break result,
            }
        };

        match connection_result {
            Err(_) => {
                tracing::warn!(attempt, "ARI websocket connection timed out");
            }
            Ok(Ok((ws_stream, _response))) => {
                tracing::info!("ARI websocket connected (unified mode)");
                state_tx.send_replace(AriConnectionState::Ready);
                let connected_at = Instant::now();

                if let Err(should_exit) = handle_connection(
                    ws_stream,
                    &event_bus,
                    &mut command_rx,
                    &mut shutdown_rx,
                    config.max_response_body_bytes,
                    config.max_websocket_message_bytes,
                )
                .await
                {
                    if should_exit {
                        state_tx.send_replace(AriConnectionState::Disconnected);
                        return;
                    }
                }

                tracing::warn!("ARI websocket disconnected (unified mode)");
                if connected_at.elapsed() >= config.reconnect.stability_window {
                    attempt = 0;
                }
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, attempt, "ARI websocket connection failed");
            }
        }

        state_tx.send_replace(AriConnectionState::Reconnecting);

        if config
            .reconnect
            .max_retries
            .is_some_and(|max| attempt >= max)
        {
            tracing::error!(
                attempt,
                "max reconnection attempts reached, stopping ws transport"
            );
            state_tx.send_replace(AriConnectionState::Terminal {
                details: format!(
                    "unified websocket connection terminated after {attempt} reconnection attempts"
                ),
            });
            return;
        }

        let delay = config.reconnect.delay_for_attempt(attempt);
        if delay > Duration::ZERO {
            tracing::info!(?delay, attempt, "waiting before reconnection");
            let sleep = tokio::time::sleep(delay);
            tokio::pin!(sleep);
            loop {
                tokio::select! {
                    biased;
                    changed = shutdown_rx.changed() => {
                        if changed.is_err() || *shutdown_rx.borrow() {
                            state_tx.send_replace(AriConnectionState::Disconnected);
                            tracing::debug!("ws transport shutting down during backoff");
                            return;
                        }
                    }
                    command = command_rx.recv() => match command { Some(command) => reject_unready(command), None => return },
                    _ = &mut sleep => break,
                }
            }
        }

        attempt = attempt.saturating_add(1);
    }
}

fn reject_unready(command: RestCommand) {
    command.lifecycle.cancel_unsent();
    let _ = command.response_tx.send(Err(AriError::RequestNotSent {
        method: command.method,
        uri: command.uri,
    }));
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
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
) -> std::result::Result<(), bool> {
    use tokio_tungstenite::tungstenite::Message;

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
                    fail_pending(&mut pending, "websocket transport shut down");
                    return Err(true);
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
                            event_bus,
                            &mut pending,
                            max_response_body_bytes,
                        );
                    }
                    Some(Ok(Message::Close(_))) => {
                        tracing::debug!("received websocket close frame");
                        fail_pending(&mut pending, "websocket closed before a response arrived");
                        return Err(false);
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "websocket read error");
                        fail_pending(&mut pending, "websocket read failed before a response arrived");
                        return Err(false);
                    }
                    None => {
                        fail_pending(&mut pending, "websocket ended before a response arrived");
                        return Err(false);
                    }
                    // ping/pong handled by tungstenite, binary frames ignored
                    _ => {}
                }
            }
            cmd = command_rx.recv(), if pending.len() < REST_COMMAND_CAPACITY => {
                match cmd {
                    Some(cmd) => {
                        // a request may time out while queued during reconnect; never
                        // execute a stale mutating operation after its caller is gone
                        if cmd.response_tx.is_closed() {
                            cmd.lifecycle.cancel_unsent();
                            tracing::debug!(
                                request_id = %cmd.request_id,
                                "discarding expired REST request"
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
                                "REST request exceeds websocket message limit"
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
                            _ = wait_for_shutdown(shutdown_rx) => {
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || "websocket transport shut down during REST request write".to_owned(),
                                );
                                let _ = cmd.response_tx.send(Err(result));
                                fail_pending(&mut pending, "websocket transport shut down");
                                return Err(true);
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
                                    "websocket write failed before a response arrived",
                                );
                                return Err(false);
                            }
                            Ok(None) => {
                                let result = write_error(
                                    &req.method,
                                    &req.uri,
                                    &cmd.request_id,
                                    &cmd.lifecycle,
                                    || "REST request was cancelled before its first wire poll".to_owned(),
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
                                    "websocket write timed out before a response arrived",
                                );
                                return Err(false);
                            }
                        }
                    }
                    None => {
                        // command channel closed — client dropped
                        fail_pending(&mut pending, "websocket request channel closed");
                        return Err(true);
                    }
                }
            }
        }
    }
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
