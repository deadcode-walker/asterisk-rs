//! AMI TCP connection management.

use crate::codec::{AmiCodec, RawAmiMessage};
use crate::error::{AmiError, Result};
use crate::event::AmiEvent;
use crate::response::{AmiResponse, PendingActions};
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::event::EventBus;

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, watch, Mutex};
use tokio_util::codec::{FramedRead, FramedWrite};

/// commands sent to the connection task
pub(crate) enum ConnectionCommand {
    /// send an action and register for its response
    SendAction {
        message: RawAmiMessage,
        action_id: String,
        response_tx: tokio::sync::oneshot::Sender<AmiResponse>,
    },
    /// graceful shutdown
    Shutdown,
}

/// manages the AMI TCP connection in a background task
pub(crate) struct ConnectionManager {
    command_tx: mpsc::Sender<ConnectionCommand>,
    state_rx: watch::Receiver<ConnectionState>,
}

impl ConnectionManager {
    /// spawn a new connection manager task
    pub fn spawn(
        address: String,
        event_bus: EventBus<AmiEvent>,
        reconnect_policy: ReconnectPolicy,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(256);
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);

        tokio::spawn(connection_task(
            address,
            command_rx,
            event_bus,
            state_tx,
            reconnect_policy,
        ));

        Self {
            command_tx,
            state_rx,
        }
    }

    /// send a command to the connection task
    pub async fn send(&self, cmd: ConnectionCommand) -> Result<()> {
        self.command_tx
            .send(cmd)
            .await
            .map_err(|_| AmiError::Disconnected)
    }

    /// get current connection state
    pub fn state(&self) -> ConnectionState {
        *self.state_rx.borrow()
    }

    /// wait for the connection state to change
    pub async fn wait_for_state(&self, target: ConnectionState) -> Result<()> {
        let mut rx = self.state_rx.clone();
        while *rx.borrow_and_update() != target {
            rx.changed().await.map_err(|_| AmiError::Disconnected)?;
        }
        Ok(())
    }

    /// shut down the connection
    pub async fn shutdown(&self) {
        let _ = self.command_tx.send(ConnectionCommand::Shutdown).await;
    }
}

async fn connection_task(
    address: String,
    mut command_rx: mpsc::Receiver<ConnectionCommand>,
    event_bus: EventBus<AmiEvent>,
    state_tx: watch::Sender<ConnectionState>,
    reconnect_policy: ReconnectPolicy,
) {
    let pending = Arc::new(Mutex::new(PendingActions::new()));
    let mut attempt: u32 = 0;

    loop {
        let _ = state_tx.send(ConnectionState::Connecting);
        tracing::info!(address = %address, attempt, "connecting to AMI");

        match TcpStream::connect(&address).await {
            Ok(stream) => {
                let _ = state_tx.send(ConnectionState::Connected);
                attempt = 0;
                tracing::info!(address = %address, "connected to AMI");

                let (read_half, write_half) = stream.into_split();
                let mut reader = FramedRead::new(read_half, AmiCodec::new());
                let mut writer = FramedWrite::new(write_half, AmiCodec::new());

                // process messages until disconnect
                loop {
                    tokio::select! {
                        // incoming message from AMI
                        frame = reader.next() => {
                            match frame {
                                Some(Ok(raw)) => {
                                    dispatch_message(raw, &pending, &event_bus).await;
                                }
                                Some(Err(e)) => {
                                    tracing::error!(error = %e, "AMI codec error");
                                    break;
                                }
                                None => {
                                    tracing::warn!("AMI connection closed");
                                    break;
                                }
                            }
                        }
                        // outbound command from client
                        cmd = command_rx.recv() => {
                            match cmd {
                                Some(ConnectionCommand::SendAction { message, action_id, response_tx }) => {
                                    pending.lock().await.register_with_sender(action_id, response_tx);
                                    if let Err(e) = writer.send(message).await {
                                        tracing::error!(error = %e, "failed to send AMI action");
                                        break;
                                    }
                                }
                                Some(ConnectionCommand::Shutdown) => {
                                    tracing::info!("AMI connection shutdown requested");
                                    let _ = state_tx.send(ConnectionState::Disconnected);
                                    return;
                                }
                                None => {
                                    // all command senders dropped
                                    let _ = state_tx.send(ConnectionState::Disconnected);
                                    return;
                                }
                            }
                        }
                    }
                }

                // connection lost — cancel pending actions
                pending.lock().await.cancel_all();
            }
            Err(e) => {
                tracing::error!(address = %address, error = %e, "failed to connect to AMI");
            }
        }

        // reconnection logic
        if reconnect_policy
            .max_retries
            .is_some_and(|max| attempt >= max)
        {
            tracing::error!("max reconnection attempts reached, giving up");
            let _ = state_tx.send(ConnectionState::Disconnected);
            return;
        }

        let _ = state_tx.send(ConnectionState::Reconnecting);
        let delay = reconnect_policy.delay_for_attempt(attempt);
        tracing::info!(?delay, attempt, "reconnecting to AMI");
        tokio::time::sleep(delay).await;
        attempt += 1;
    }
}

async fn dispatch_message(
    raw: RawAmiMessage,
    pending: &Arc<Mutex<PendingActions>>,
    event_bus: &EventBus<AmiEvent>,
) {
    // try as response first
    if let Some(response) = AmiResponse::from_raw(&raw) {
        let action_id = response.action_id.clone();
        if !pending.lock().await.deliver(response) {
            tracing::debug!(action_id, "received response for unknown action");
        }
        return;
    }

    // try as event
    if let Some(event) = AmiEvent::from_raw(&raw) {
        tracing::trace!(event = event.event_name(), "AMI event received");
        event_bus.publish(event);
        return;
    }

    tracing::debug!("received unclassifiable AMI message");
}
