//! WebSocket event listener with automatic reconnection.

use std::time::Duration;

use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventBus;
use futures_util::{SinkExt, StreamExt};
use tokio::sync::watch;

use crate::event::AriMessage;
use crate::util::redact_url;

/// background websocket listener that connects to the ARI event stream,
/// deserializes events, and publishes them to an event bus
pub(crate) struct WsEventListener {
    shutdown_tx: watch::Sender<bool>,
    task_handle: tokio::task::JoinHandle<()>,
}

impl WsEventListener {
    /// spawn the websocket listener as a background task
    pub(crate) fn spawn(
        ws_url: String,
        event_bus: EventBus<AriMessage>,
        reconnect: ReconnectPolicy,
    ) -> Self {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let task_handle = tokio::spawn(ws_loop(ws_url, event_bus, reconnect, shutdown_rx));

        Self {
            shutdown_tx,
            task_handle,
        }
    }

    /// signal the background task to shut down
    pub(crate) fn shutdown(&self) {
        let _ = self.shutdown_tx.send(true);
        self.task_handle.abort();
    }
}

impl Drop for WsEventListener {
    fn drop(&mut self) {
        self.shutdown();
    }
}

/// main websocket loop with reconnection logic
async fn ws_loop(
    ws_url: String,
    event_bus: EventBus<AriMessage>,
    reconnect: ReconnectPolicy,
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
            tokio_tungstenite::connect_async(&ws_url),
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
                _ = tokio::time::sleep(delay) => {}
                _ = shutdown_rx.changed() => {
                    if *shutdown_rx.borrow() {
                        tracing::debug!("websocket listener shutting down during backoff");
                        return;
                    }
                }
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
            msg = read.next() => {
                match msg {
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
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    if let Err(e) = write.send(tokio_tungstenite::tungstenite::Message::Close(None)).await {
                        tracing::debug!(error = %e, "failed to send websocket close frame");
                    }
                    return Err(true);
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
                tracing::warn!(error = %e, "failed to deserialize ARI event");
                tracing::trace!(payload = %text, "raw ARI event payload");
            }
        },
        Message::Close(_) => {
            tracing::debug!("received websocket close frame");
        }
        // ping/pong handled by tungstenite automatically, binary frames ignored
        _ => {}
    }
}
