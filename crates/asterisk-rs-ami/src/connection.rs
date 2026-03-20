//! AMI TCP connection management.

use crate::action::{AmiAction, ChallengeAction, ChallengeLoginAction, LoginAction, PingAction};
use crate::codec::{AmiCodec, RawAmiMessage};
use crate::error::{AmiError, Result};
use crate::event::AmiEvent;
use crate::response::{AmiResponse, PendingActions};
use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::event::EventBus;

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
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
    /// send an action that returns events as its response
    SendEventGeneratingAction {
        message: RawAmiMessage,
        action_id: String,
        response_tx: tokio::sync::oneshot::Sender<crate::response::EventListResponse>,
    },
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
        credentials: Credentials,
        event_bus: EventBus<AmiEvent>,
        reconnect_policy: ReconnectPolicy,
        ping_interval: Option<Duration>,
    ) -> Self {
        let (command_tx, command_rx) = mpsc::channel(256);
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);

        tokio::spawn(connection_task(
            address,
            credentials,
            command_rx,
            event_bus,
            state_tx,
            reconnect_policy,
            ping_interval,
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
    credentials: Credentials,
    mut command_rx: mpsc::Receiver<ConnectionCommand>,
    event_bus: EventBus<AmiEvent>,
    state_tx: watch::Sender<ConnectionState>,
    reconnect_policy: ReconnectPolicy,
    ping_interval: Option<Duration>,
) {
    let pending = Arc::new(Mutex::new(PendingActions::new()));
    let mut attempt: u32 = 0;

    loop {
        let _ = state_tx.send(ConnectionState::Connecting);
        tracing::info!(address = %address, attempt, "connecting to AMI");

        match tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(&address)).await {
            Ok(Ok(stream)) => {
                tracing::info!(address = %address, "TCP connected to AMI");

                let (read_half, write_half) = stream.into_split();
                let mut reader = FramedRead::new(read_half, AmiCodec::new());
                let mut writer = FramedWrite::new(write_half, AmiCodec::new());

                // authenticate after connecting
                if let Err(e) = perform_login(&credentials, &mut reader, &mut writer).await {
                    tracing::error!(error = %e, "AMI login failed after connect");
                    continue; // will trigger reconnect
                }
                tracing::info!("AMI login successful");
                attempt = 0; // reset only after successful auth
                let _ = state_tx.send(ConnectionState::Connected);

                // set up keep-alive ping timer
                let mut ping_timer = ping_interval.map(tokio::time::interval);
                if let Some(ref mut timer) = ping_timer {
                    timer.tick().await; // consume the immediate first tick
                }

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
                                Some(ConnectionCommand::SendEventGeneratingAction { message, action_id, response_tx }) => {
                                    pending.lock().await.register_event_list(action_id, response_tx);
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
                        // keep-alive ping
                        _ = async {
                            match ping_timer.as_mut() {
                                Some(timer) => timer.tick().await,
                                None => std::future::pending().await,
                            }
                        } => {
                            let ping = PingAction;
                            let (_, ping_msg) = ping.to_message();
                            if let Err(e) = writer.send(ping_msg).await {
                                tracing::warn!(error = %e, "keep-alive ping failed, reconnecting");
                                break;
                            }
                            tracing::trace!("keep-alive ping sent");
                        }
                    }
                }

                // connection lost — cancel pending actions
                pending.lock().await.cancel_all();
            }
            Ok(Err(e)) => {
                tracing::error!(address = %address, error = %e, "failed to connect to AMI");
            }
            Err(_) => {
                tracing::error!(address = %address, "AMI connection timed out");
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
        // poll shutdown during the reconnect sleep so we exit promptly
        tokio::select! {
            () = tokio::time::sleep(delay) => {}
            cmd = command_rx.recv() => {
                match cmd {
                    None | Some(ConnectionCommand::Shutdown) => {
                        tracing::info!("shutdown received during reconnect backoff");
                        let _ = state_tx.send(ConnectionState::Disconnected);
                        return;
                    }
                    Some(_) => {
                        // non-shutdown command while disconnected; it will time out on the
                        // caller side — nothing we can do until reconnected
                    }
                }
            }
        }
        attempt += 1;
    }
}

/// perform the AMI login sequence over the raw framed connection
///
/// tries MD5 challenge-response first, falls back to plaintext
async fn perform_login(
    credentials: &Credentials,
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, AmiCodec>,
    writer: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, AmiCodec>,
) -> Result<()> {
    // try MD5 challenge-response first
    let (_, challenge_msg) = ChallengeAction.to_message();
    writer.send(challenge_msg).await?;

    let challenge_resp = read_next_response(reader).await?;

    if challenge_resp.success {
        if let Some(challenge) = challenge_resp.get("Challenge") {
            let key = compute_md5_key(challenge, credentials.secret());
            let login = ChallengeLoginAction {
                username: credentials.username().to_string(),
                key,
            };
            let (_, login_msg) = login.to_message();
            writer.send(login_msg).await?;

            let login_resp = read_next_response(reader).await?;
            if !login_resp.success {
                return Err(AmiError::Auth(
                    asterisk_rs_core::error::AuthError::Rejected {
                        reason: login_resp.message.unwrap_or_default(),
                    },
                ));
            }
            return Ok(());
        }
    }

    // fall back to plaintext
    let login = LoginAction {
        username: credentials.username().to_string(),
        secret: credentials.secret().to_string(),
    };
    let (_, login_msg) = login.to_message();
    writer.send(login_msg).await?;

    let login_resp = read_next_response(reader).await?;
    if !login_resp.success {
        return Err(AmiError::Auth(
            asterisk_rs_core::error::AuthError::Rejected {
                reason: login_resp.message.unwrap_or_default(),
            },
        ));
    }
    Ok(())
}

/// read frames until we get a Response (skipping events and banners)
async fn read_next_response(
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, AmiCodec>,
) -> Result<AmiResponse> {
    loop {
        match reader.next().await {
            Some(Ok(raw)) => {
                if let Some(resp) = AmiResponse::from_raw(&raw) {
                    return Ok(resp);
                }
                // skip events/banners during login
            }
            Some(Err(e)) => return Err(e),
            None => return Err(AmiError::Disconnected),
        }
    }
}

fn compute_md5_key(challenge: &str, secret: &str) -> String {
    use md5::{Digest, Md5};
    let mut hasher = Md5::new();
    hasher.update(challenge.as_bytes());
    hasher.update(secret.as_bytes());
    format!("{:x}", hasher.finalize())
}

async fn dispatch_message(
    raw: RawAmiMessage,
    pending: &Arc<Mutex<PendingActions>>,
    event_bus: &EventBus<AmiEvent>,
) {
    // try as response first
    if let Some(response) = AmiResponse::from_raw(&raw) {
        let mut guard = pending.lock().await;

        // check if this is for an event-generating action
        if guard.deliver_event_list_response(response.clone()) {
            return;
        }

        // regular action response
        let action_id = response.action_id.clone();
        if !guard.deliver(response) {
            tracing::debug!(action_id, "received response for unknown action");
        }
        return;
    }

    // try as event
    if let Some(event) = AmiEvent::from_raw(&raw) {
        // check if event has an ActionID matching a pending event list
        if let Some(aid) = raw.get("ActionID") {
            let mut guard = pending.lock().await;
            if guard.deliver_event_list_event(aid, event.clone()) {
                // also publish to event bus so subscribers see it
                event_bus.publish(event);
                return;
            }
        }

        tracing::trace!(event = event.event_name(), "AMI event received");
        event_bus.publish(event);
        return;
    }

    tracing::debug!("received unclassifiable AMI message");
}
