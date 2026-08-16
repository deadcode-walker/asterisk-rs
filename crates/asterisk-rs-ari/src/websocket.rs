//! WebSocket event listener with automatic reconnection.

use std::sync::Arc;
use std::sync::Mutex;
use std::time::Duration;

use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventBus;
use futures_util::{SinkExt, StreamExt};
use rustls_platform_verifier::BuilderVerifierExt;
use tokio::sync::watch;

use crate::error::{AriError, Result};
use crate::event::AriMessage;
use crate::util::redact_url;

const TASK_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(1);

/// Owned background task that drains cooperatively, then aborts after a bound.
pub(crate) struct OwnedTask {
    handle: Mutex<Option<tokio::task::JoinHandle<()>>>,
}

/// Keeps a taken task owned even if its async shutdown future is cancelled.
struct AbortOnDrop(Option<tokio::task::JoinHandle<()>>);

impl AbortOnDrop {
    fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self(Some(handle))
    }

    fn handle_mut(&mut self) -> &mut tokio::task::JoinHandle<()> {
        self.0.as_mut().expect("guard always owns a handle")
    }

    fn disarm(&mut self) {
        self.0.take();
    }
}

impl Drop for AbortOnDrop {
    fn drop(&mut self) {
        if let Some(handle) = self.0.take() {
            handle.abort();
        }
    }
}

impl OwnedTask {
    pub(crate) fn new(handle: tokio::task::JoinHandle<()>) -> Self {
        Self {
            handle: Mutex::new(Some(handle)),
        }
    }

    fn take(&self) -> Option<tokio::task::JoinHandle<()>> {
        let mut guard = self
            .handle
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        guard.take()
    }

    pub(crate) async fn shutdown_and_wait(&self, task_name: &'static str) {
        let Some(handle) = self.take() else {
            return;
        };
        let mut handle = AbortOnDrop::new(handle);

        match tokio::time::timeout(TASK_SHUTDOWN_TIMEOUT, handle.handle_mut()).await {
            Ok(result) => report_task_result(task_name, result),
            Err(_) => {
                tracing::warn!(task = task_name, "aborting ARI task after shutdown timeout");
                handle.handle_mut().abort();
                report_task_result(task_name, handle.handle_mut().await);
            }
        }
        handle.disarm();
    }

    pub(crate) fn abort(&self) {
        if let Some(handle) = self.take() {
            handle.abort();
        }
    }
}

impl Drop for OwnedTask {
    fn drop(&mut self) {
        self.abort();
    }
}

fn report_task_result(
    task_name: &'static str,
    result: std::result::Result<(), tokio::task::JoinError>,
) {
    if let Err(error) = result {
        if error.is_panic() {
            tracing::error!(task = task_name, error = %error, "ARI background task panicked");
        }
    }
}

/// Build the explicit TLS connector shared by every outbound ARI WebSocket.
///
/// Supplying the provider directly prevents downstream feature unification
/// from making rustls choose between AWS-LC and another compiled provider.
fn platform_tls_connector() -> Result<tokio_tungstenite::Connector> {
    let provider = Arc::new(rustls::crypto::aws_lc_rs::default_provider());
    let config = rustls::ClientConfig::builder_with_provider(provider)
        .with_safe_default_protocol_versions()
        .map_err(|error| {
            AriError::WebSocket(format!(
                "failed to select websocket TLS protocol versions: {error}"
            ))
        })?
        .with_platform_verifier()
        .map_err(|error| {
            AriError::WebSocket(format!(
                "failed to configure websocket platform verifier: {error}"
            ))
        })?
        .with_no_client_auth();

    Ok(tokio_tungstenite::Connector::Rustls(Arc::new(config)))
}

pub(crate) fn connector_for_url(url: &str) -> Result<tokio_tungstenite::Connector> {
    match url::Url::parse(url)
        .map_err(|error| AriError::InvalidUrl(error.to_string()))?
        .scheme()
    {
        "ws" => Ok(tokio_tungstenite::Connector::Plain),
        "wss" => platform_tls_connector(),
        scheme => Err(AriError::InvalidUrl(format!(
            "unsupported websocket URL scheme: {scheme}"
        ))),
    }
}

pub(crate) fn websocket_config(
    max_message_bytes: usize,
) -> tokio_tungstenite::tungstenite::protocol::WebSocketConfig {
    // Leave room for WebSocket framing around one maximum-sized message. REST
    // actors preflight their serialized envelopes against max_message_bytes,
    // so this is a bounded transport buffer rather than an implicit request
    // size check with ambiguous execution semantics.
    let max_write_buffer_bytes = max_message_bytes.saturating_add(64);
    tokio_tungstenite::tungstenite::protocol::WebSocketConfig::default()
        .read_buffer_size(16 * 1024)
        .write_buffer_size(0)
        .max_write_buffer_size(max_write_buffer_bytes)
        .max_message_size(Some(max_message_bytes))
        .max_frame_size(Some(max_message_bytes))
}

/// background websocket listener that connects to the ARI event stream,
/// deserializes events, and publishes them to an event bus
pub(crate) struct WsEventListener {
    shutdown_tx: watch::Sender<bool>,
    task: OwnedTask,
}

impl WsEventListener {
    /// spawn the websocket listener as a background task
    pub(crate) fn spawn(
        ws_url: String,
        event_bus: EventBus<AriMessage>,
        reconnect: ReconnectPolicy,
        max_websocket_message_bytes: usize,
    ) -> Result<Self> {
        let tls_connector = connector_for_url(&ws_url)?;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(ws_loop(
            ws_url,
            event_bus,
            reconnect,
            tls_connector,
            max_websocket_message_bytes,
            shutdown_rx,
        ));

        Ok(Self {
            shutdown_tx,
            task: OwnedTask::new(task_handle),
        })
    }

    /// signal the background task to shut down
    pub(crate) async fn shutdown_and_wait(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task
            .shutdown_and_wait("ARI event websocket listener")
            .await;
    }

    pub(crate) fn abort(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task.abort();
    }
}

impl Drop for WsEventListener {
    fn drop(&mut self) {
        self.abort();
    }
}

/// main websocket loop with reconnection logic
async fn ws_loop(
    ws_url: String,
    event_bus: EventBus<AriMessage>,
    reconnect: ReconnectPolicy,
    tls_connector: tokio_tungstenite::Connector,
    max_websocket_message_bytes: usize,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut attempt: u32 = 0;

    loop {
        if *shutdown_rx.borrow() {
            tracing::debug!("websocket listener shutting down");
            return;
        }

        tracing::info!(url = %redact_url(&ws_url), attempt, "connecting to ARI websocket");

        match tokio::time::timeout(
            Duration::from_secs(10),
            tokio_tungstenite::connect_async_tls_with_config(
                &ws_url,
                Some(websocket_config(max_websocket_message_bytes)),
                false,
                Some(tls_connector.clone()),
            ),
        )
        .await
        {
            Err(_) => {
                tracing::warn!(attempt, "ARI websocket connection timed out");
            }
            Ok(Err(e)) => {
                tracing::warn!(error = %e, attempt, "ARI websocket connection failed");
            }
            Ok(Ok((ws_stream, _response))) => {
                tracing::info!("ARI websocket connected");
                // reset attempt counter on successful connection
                attempt = 0;

                if let Err(should_exit) =
                    read_messages(ws_stream, &event_bus, &mut shutdown_rx).await
                {
                    if should_exit {
                        return;
                    }
                }

                tracing::warn!("ARI websocket disconnected");
            }
        }

        // check if we've exhausted retries
        if reconnect.max_retries.is_some_and(|max| attempt >= max) {
            tracing::error!(
                attempt,
                "max reconnection attempts reached, stopping websocket listener"
            );
            return;
        }

        let delay = reconnect.delay_for_attempt(attempt);
        if delay > Duration::ZERO {
            tracing::info!(?delay, attempt, "waiting before reconnection");

            // wait for delay or shutdown signal
            tokio::select! {
                biased;
                changed = shutdown_rx.changed() => {
                    if changed.is_err() || *shutdown_rx.borrow() {
                        tracing::debug!("websocket listener shutting down during backoff");
                        return;
                    }
                }
                _ = tokio::time::sleep(delay) => {}
            }
        }

        attempt = attempt.saturating_add(1);
    }
}

/// read messages from an active websocket connection
///
/// returns `Err(true)` if shutdown was requested, `Err(false)` on disconnect
async fn read_messages(
    ws_stream: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
    event_bus: &EventBus<AriMessage>,
    shutdown_rx: &mut watch::Receiver<bool>,
) -> std::result::Result<(), bool> {
    let (mut write, mut read) = ws_stream.split();

    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Close(None)).await {
                        tracing::debug!(error = %e, "failed to send websocket close frame");
                    }
                    return Err(true);
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(tokio_tungstenite::tungstenite::Message::Close(_))) => {
                        tracing::debug!("received websocket close frame");
                        return Err(false);
                    }
                    Some(Ok(message)) => {
                        handle_message(message, event_bus);
                    }
                    Some(Err(e)) => {
                        tracing::warn!(error = %e, "websocket read error");
                        return Err(false);
                    }
                    None => {
                        // stream ended
                        return Err(false);
                    }
                }
            }
        }
    }
}

/// process a single websocket message
fn handle_message(
    message: tokio_tungstenite::tungstenite::Message,
    event_bus: &EventBus<AriMessage>,
) {
    use tokio_tungstenite::tungstenite::Message;

    match message {
        Message::Text(text) => match serde_json::from_str::<AriMessage>(&text) {
            Ok(event) => {
                tracing::debug!(?event, "received ARI event");
                event_bus.publish(event);
            }
            Err(e) => {
                tracing::warn!(
                    error = %e,
                    payload_bytes = text.len(),
                    "failed to deserialize ARI event"
                );
            }
        },
        Message::Close(_) => {
            tracing::debug!("received websocket close frame");
        }
        // ping/pong handled by tungstenite automatically, binary frames ignored
        _ => {}
    }
}
