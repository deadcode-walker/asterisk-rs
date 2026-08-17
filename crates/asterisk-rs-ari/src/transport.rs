//! REST transport abstraction for HTTP and WebSocket modes.

use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicU8, Ordering};
use std::time::Duration;

use tokio::sync::{oneshot, watch};
use tokio::time::Instant;

use crate::config::AriConfig;
use crate::error::{AriError, HttpError, Result};
use crate::event::{AriEvent, AriMessage};
use crate::websocket::WsEventListener;
use crate::ws_transport::WsTransport;
use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::event::EventBus;

const REQUEST_QUEUED: u8 = 0;
const REQUEST_WRITING: u8 = 1;
const REQUEST_WRITTEN: u8 = 2;
const REQUEST_CANCELLED: u8 = 3;
static HTTP_REQUEST_COUNTER: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(1);
pub(crate) const REST_COMMAND_CAPACITY: usize = 64;

pub(crate) struct RestCommand {
    pub request_id: String,
    pub method: String,
    pub uri: String,
    pub content_type: Option<String>,
    pub message_body: Option<String>,
    pub deadline: Instant,
    pub lifecycle: Arc<RequestLifecycle>,
    pub response_tx: oneshot::Sender<Result<TransportResponse>>,
}

pub(crate) struct PendingResponse {
    pub deadline: Instant,
    pub request_id: String,
    pub method: String,
    pub uri: String,
    pub lifecycle: Arc<RequestLifecycle>,
    pub response_tx: oneshot::Sender<Result<TransportResponse>>,
}

pub(crate) fn fail_pending(pending: &mut HashMap<String, PendingResponse>, details: &str) {
    for (_, response) in pending.drain() {
        let error = write_error(
            &response.method,
            &response.uri,
            &response.request_id,
            &response.lifecycle,
            || details.to_owned(),
        );
        let _ = response.response_tx.send(Err(error));
    }
}

pub(crate) fn purge_expired(pending: &mut HashMap<String, PendingResponse>) {
    let now = Instant::now();
    let expired: Vec<_> = pending
        .iter()
        .filter(|(_, response)| response.response_tx.is_closed() || response.deadline <= now)
        .map(|(request_id, _)| request_id.clone())
        .collect();
    for request_id in expired {
        let Some(response) = pending.remove(&request_id) else {
            continue;
        };
        if response.response_tx.is_closed() {
            tracing::debug!(%request_id, "discarding cancelled REST response correlation");
            continue;
        }
        let error = deadline_error(
            &response.method,
            &response.uri,
            &response.request_id,
            &response.lifecycle,
        );
        let _ = response.response_tx.send(Err(error));
    }
}

pub(crate) fn route_text_message(
    text: &str,
    event_bus: &EventBus<AriMessage>,
    pending: &mut HashMap<String, PendingResponse>,
    max_response_body_bytes: usize,
) {
    match serde_json::from_str::<AriMessage>(text) {
        Ok(msg) => {
            if let AriEvent::RESTResponse {
                ref request_id,
                status_code,
                ref reason_phrase,
                ref message_body,
                ..
            } = msg.event
            {
                if let Some(response) = pending.remove(request_id) {
                    if let Some(body) = message_body {
                        if body.len() > max_response_body_bytes {
                            let _ = response.response_tx.send(Err(AriError::ResponseTooLarge {
                                limit: max_response_body_bytes,
                                received: u64::try_from(body.len()).unwrap_or(u64::MAX),
                            }));
                            return;
                        }
                    }
                    let result = u16::try_from(status_code)
                        .map_err(|_| {
                            AriError::WebSocket(format!(
                                "invalid REST response status code: {status_code}"
                            ))
                        })
                        .and_then(|status| {
                            TransportResponse {
                                status,
                                body: message_body.clone(),
                            }
                            .require_success_with_fallback(
                                (!reason_phrase.is_empty()).then(|| reason_phrase.clone()),
                            )
                        });
                    let _ = response.response_tx.send(result);
                }
            } else {
                event_bus.publish(msg);
            }
        }
        Err(_) => tracing::warn!(
            payload_bytes = text.len(),
            "failed to deserialize ARI message"
        ),
    }
}

/// Observable lifecycle of the ARI event WebSocket.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum AriConnectionState {
    Connecting,
    Ready,
    Reconnecting,
    Terminal { details: String },
    Disconnected,
}

pub(crate) async fn wait_until_ready(
    state: &mut watch::Receiver<AriConnectionState>,
    deadline: tokio::time::Instant,
) -> Result<()> {
    tokio::time::timeout_at(deadline, async {
        loop {
            match state.borrow().clone() {
                AriConnectionState::Ready => return Ok(()),
                AriConnectionState::Terminal { details } => {
                    return Err(AriError::WebSocket(details));
                }
                AriConnectionState::Disconnected => return Err(AriError::Disconnected),
                AriConnectionState::Connecting | AriConnectionState::Reconnecting => {}
            }
            state.changed().await.map_err(|_| AriError::Disconnected)?;
        }
    })
    .await
    .map_err(|_| AriError::WebSocket("initial websocket readiness timed out".to_owned()))?
}

/// shared state that makes caller timeout and actor write admission atomic
#[derive(Debug, Default)]
pub(crate) struct RequestLifecycle {
    state: AtomicU8,
}

impl RequestLifecycle {
    /// mark the exact first poll of the sink write future
    fn begin_wire_poll(&self) -> bool {
        self.state
            .compare_exchange(
                REQUEST_QUEUED,
                REQUEST_WRITING,
                Ordering::AcqRel,
                Ordering::Acquire,
            )
            .is_ok()
    }

    pub(crate) fn mark_written(&self) {
        self.state.store(REQUEST_WRITTEN, Ordering::Release);
    }

    /// cancel while the request is still definitely absent from the wire
    pub(crate) fn cancel_unsent(&self) -> bool {
        loop {
            let state = self.state.load(Ordering::Acquire);
            if state != REQUEST_QUEUED {
                return false;
            }
            if self
                .state
                .compare_exchange(
                    state,
                    REQUEST_CANCELLED,
                    Ordering::AcqRel,
                    Ordering::Acquire,
                )
                .is_ok()
            {
                return true;
            }
        }
    }

    pub(crate) fn may_have_executed(&self) -> bool {
        matches!(
            self.state.load(Ordering::Acquire),
            REQUEST_WRITING | REQUEST_WRITTEN
        )
    }
}

/// Poll a sink write only after atomically crossing the may-have-written boundary.
///
/// `None` means the caller cancelled the request before the first sink poll, so
/// no bytes from this request can have reached the wire.
pub(crate) async fn poll_wire_write<F>(lifecycle: &RequestLifecycle, future: F) -> Option<F::Output>
where
    F: std::future::Future,
{
    let mut future = std::pin::pin!(future);
    let mut first_poll = true;
    std::future::poll_fn(|cx| {
        if first_poll {
            first_poll = false;
            if !lifecycle.begin_wire_poll() {
                return std::task::Poll::Ready(None);
            }
        }
        future.as_mut().poll(cx).map(Some)
    })
    .await
}

pub(crate) fn is_mutating(method: &str) -> bool {
    matches!(method, "POST" | "PUT" | "DELETE" | "PATCH")
}

pub(crate) fn deadline_error(
    method: &str,
    uri: &str,
    request_id: &str,
    lifecycle: &RequestLifecycle,
) -> AriError {
    if lifecycle.cancel_unsent() {
        return AriError::RequestNotSent {
            method: method.to_owned(),
            uri: uri.to_owned(),
        };
    }
    if is_mutating(method) && lifecycle.may_have_executed() {
        return AriError::OutcomeUnknown {
            request_id: request_id.to_owned(),
            method: method.to_owned(),
            uri: uri.to_owned(),
        };
    }

    AriError::WebSocket(format!("{method} {uri} timed out"))
}

pub(crate) fn write_error(
    method: &str,
    uri: &str,
    request_id: &str,
    lifecycle: &RequestLifecycle,
    details: impl FnOnce() -> String,
) -> AriError {
    if lifecycle.cancel_unsent() {
        return AriError::RequestNotSent {
            method: method.to_owned(),
            uri: uri.to_owned(),
        };
    }
    if is_mutating(method) {
        AriError::OutcomeUnknown {
            request_id: request_id.to_owned(),
            method: method.to_owned(),
            uri: uri.to_owned(),
        }
    } else {
        AriError::WebSocket(details())
    }
}

pub(crate) fn outbound_message_limit_error(
    method: &str,
    uri: &str,
    message_bytes: usize,
    limit: usize,
) -> Option<AriError> {
    if message_bytes <= limit {
        return None;
    }

    if is_mutating(method) {
        Some(AriError::RequestNotSent {
            method: method.to_owned(),
            uri: uri.to_owned(),
        })
    } else {
        Some(AriError::WebSocket(format!(
            "serialized {method} {uri} request is {message_bytes} bytes, exceeding the websocket message limit of {limit} bytes"
        )))
    }
}

/// response from a transport REST operation
pub(crate) struct TransportResponse {
    pub status: u16,
    pub body: Option<String>,
}

impl TransportResponse {
    pub(crate) fn require_success(self) -> Result<Self> {
        self.require_success_with_fallback(None)
    }

    pub(crate) fn require_success_with_fallback(self, fallback: Option<String>) -> Result<Self> {
        if (200..300).contains(&self.status) {
            return Ok(self);
        }

        let Self { status, body } = self;
        let message = body
            .or(fallback)
            .unwrap_or_else(|| format!("HTTP {status}"));
        Err(AriError::Api { status, message })
    }
}

/// internal transport implementation — dispatches REST calls to either
/// HTTP (reqwest) or a unified WebSocket connection
pub(crate) enum TransportInner {
    Http(HttpTransport),
    WebSocket(WsTransport),
}

impl TransportInner {
    pub(crate) fn connection_state(&self) -> AriConnectionState {
        match self {
            Self::Http(t) => t.ws_listener.connection_state(),
            Self::WebSocket(t) => t.connection_state(),
        }
    }

    pub(crate) fn subscribe_connection_state(&self) -> watch::Receiver<AriConnectionState> {
        match self {
            Self::Http(t) => t.ws_listener.subscribe_connection_state(),
            Self::WebSocket(t) => t.subscribe_connection_state(),
        }
    }

    pub(crate) async fn wait_ready(&self, timeout: Duration) -> Result<()> {
        let deadline = tokio::time::Instant::now() + timeout;
        let mut state = self.subscribe_connection_state();
        wait_until_ready(&mut state, deadline).await
    }
    pub(crate) async fn request(
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

    pub(crate) fn shutdown(&self) {
        match self {
            Self::Http(t) => t.ws_listener.abort(),
            Self::WebSocket(t) => t.abort(),
        }
    }

    pub(crate) async fn shutdown_and_wait(&self) {
        match self {
            Self::Http(t) => t.ws_listener.shutdown_and_wait().await,
            Self::WebSocket(t) => t.shutdown_and_wait().await,
        }
    }
}

/// HTTP-based transport — uses reqwest for REST and a separate
/// WebSocket listener for events
pub(crate) struct HttpTransport {
    client: reqwest::Client,
    base_url: String,
    credentials: Credentials,
    ws_listener: WsEventListener,
    max_response_body_bytes: usize,
}

impl HttpTransport {
    pub(crate) fn new(config: &AriConfig, event_bus: EventBus<AriMessage>) -> Result<Self> {
        let mut client_builder = reqwest::Client::builder()
            .connect_timeout(Duration::from_secs(10))
            .timeout(config.request_timeout())
            .redirect(reqwest::redirect::Policy::none());
        for root in &config.tls_trust.reqwest_roots {
            client_builder = client_builder.add_root_certificate(root.clone());
        }
        let client = client_builder
            .build()
            .map_err(|error| AriError::Http(HttpError::new(error)))?;

        let ws_listener = WsEventListener::spawn(
            config.ws_url().to_string(),
            event_bus,
            config.reconnect_policy().clone(),
            config.max_websocket_message_bytes(),
            &config.tls_trust.rustls_roots,
        )?;

        Ok(Self {
            client,
            base_url: config.base_url().as_str().trim_end_matches('/').to_owned(),
            credentials: config.credentials().clone(),
            ws_listener,
            max_response_body_bytes: config.max_response_body_bytes(),
        })
    }

    pub(crate) async fn request(
        &self,
        method: &str,
        path: &str,
        body: Option<String>,
    ) -> Result<TransportResponse> {
        let request_id = format!(
            "http-{}",
            HTTP_REQUEST_COUNTER.fetch_add(1, Ordering::Relaxed)
        );
        let url = format!("{}/{}", self.base_url, path.trim_start_matches('/'));
        let http_method = parse_method(method)?;

        let mut req = self
            .client
            .request(http_method, &url)
            .basic_auth(self.credentials.username(), Some(self.credentials.secret()));

        if let Some(json_body) = body {
            req = req
                .header(reqwest::header::CONTENT_TYPE, "application/json")
                .body(json_body);
        }

        let response = match req.send().await {
            Ok(response) => response,
            Err(error) if is_mutating(method) && error.is_connect() => {
                return Err(AriError::RequestNotSent {
                    method: method.to_owned(),
                    uri: path.to_owned(),
                });
            }
            Err(_error) if is_mutating(method) => {
                return Err(AriError::OutcomeUnknown {
                    request_id,
                    method: method.to_owned(),
                    uri: path.to_owned(),
                });
            }
            Err(error) => return Err(AriError::Http(HttpError::new(error))),
        };
        let status = response.status().as_u16();
        let body = read_response_body(response, self.max_response_body_bytes).await?;
        TransportResponse { status, body }.require_success()
    }
}

async fn read_response_body(
    mut response: reqwest::Response,
    limit: usize,
) -> Result<Option<String>> {
    if let Some(content_length) = response.content_length() {
        let limit_u64 = u64::try_from(limit).unwrap_or(u64::MAX);
        if content_length > limit_u64 {
            return Err(AriError::ResponseTooLarge {
                limit,
                received: content_length,
            });
        }
    }

    let capacity = response
        .content_length()
        .and_then(|length| usize::try_from(length).ok())
        .unwrap_or(0)
        .min(limit);
    let mut body = Vec::with_capacity(capacity);
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|error| AriError::Http(HttpError::new(error)))?
    {
        let received = body.len().saturating_add(chunk.len());
        if received > limit {
            return Err(AriError::ResponseTooLarge {
                limit,
                received: u64::try_from(received).unwrap_or(u64::MAX),
            });
        }
        body.extend_from_slice(&chunk);
    }

    if body.is_empty() {
        Ok(None)
    } else {
        Ok(Some(String::from_utf8_lossy(&body).into_owned()))
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
