//! AMI TCP connection management.

use crate::action::{AmiAction, ChallengeAction, ChallengeLoginAction, LoginAction, PingAction};
use crate::codec::{AmiCodec, RawAmiMessage};
use crate::error::{AmiError, Result};
use crate::event::AmiEvent;
use crate::response::{
    AmiResponse, EventListTerminal, MAX_IN_FLIGHT_ACTIONS, MAX_IN_FLIGHT_EVENT_LISTS,
    PendingActions, RequestLifecycle,
};
use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::event::EventBus;

use futures_util::{SinkExt, StreamExt};
use std::sync::Arc;
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::sync::{mpsc, oneshot, watch};
use tokio::time::Instant;
use tokio::time::MissedTickBehavior;
use tokio_util::codec::{FramedRead, FramedWrite};
use zeroize::Zeroizing;

/// commands sent to the connection task
pub(crate) enum ConnectionCommand {
    /// send an action and register for its response
    SendAction {
        message: RawAmiMessage,
        action_id: String,
        deadline: Instant,
        timeout: Duration,
        lifecycle: Arc<RequestLifecycle>,
        response_tx: oneshot::Sender<Result<AmiResponse>>,
    },
    /// graceful shutdown
    Shutdown,
    /// send an action that returns events as its response
    SendEventGeneratingAction {
        message: RawAmiMessage,
        action_id: String,
        deadline: Instant,
        timeout: Duration,
        lifecycle: Arc<RequestLifecycle>,
        response_tx: oneshot::Sender<Result<crate::response::EventListResponse>>,
    },
}

/// manages the AMI TCP connection in a background task
pub(crate) struct ConnectionManager {
    command_tx: mpsc::Sender<ConnectionCommand>,
    state_rx: watch::Receiver<ConnectionState>,
    task: tokio::task::AbortHandle,
}

impl ConnectionManager {
    /// spawn a new connection manager task
    pub(crate) fn spawn(
        address: String,
        credentials: Credentials,
        event_bus: EventBus<AmiEvent>,
        reconnect_policy: ReconnectPolicy,
        ping_interval: Option<Duration>,
        require_challenge: bool,
    ) -> (Self, oneshot::Receiver<Result<()>>) {
        let (command_tx, command_rx) = mpsc::channel(256);
        let (state_tx, state_rx) = watch::channel(ConnectionState::Disconnected);
        let (startup_tx, startup_rx) = oneshot::channel();

        let task = tokio::spawn(connection_task(
            address,
            credentials,
            command_rx,
            event_bus,
            state_tx,
            startup_tx,
            reconnect_policy,
            ping_interval,
            require_challenge,
        ))
        .abort_handle();

        (
            Self {
                command_tx,
                state_rx,
                task,
            },
            startup_rx,
        )
    }

    /// send a command to the connection task
    pub(crate) async fn send(
        &self,
        cmd: ConnectionCommand,
        deadline: Instant,
        timeout: Duration,
    ) -> Result<()> {
        tokio::time::timeout_at(deadline, self.command_tx.send(cmd))
            .await
            .map_err(|_| {
                AmiError::Timeout(asterisk_rs_core::error::TimeoutError::Action {
                    elapsed: timeout,
                })
            })?
            .map_err(|_| AmiError::Disconnected)
    }

    /// get current connection state
    pub(crate) fn state(&self) -> ConnectionState {
        *self.state_rx.borrow()
    }

    /// shut down the connection
    pub(crate) async fn shutdown(&self) {
        let _ = self.command_tx.send(ConnectionCommand::Shutdown).await;
    }
}

impl Drop for ConnectionManager {
    fn drop(&mut self) {
        self.task.abort();
    }
}

#[allow(clippy::too_many_arguments)]
async fn connection_task(
    address: String,
    credentials: Credentials,
    mut command_rx: mpsc::Receiver<ConnectionCommand>,
    event_bus: EventBus<AmiEvent>,
    state_tx: watch::Sender<ConnectionState>,
    startup_tx: oneshot::Sender<Result<()>>,
    reconnect_policy: ReconnectPolicy,
    ping_interval: Option<Duration>,
    require_challenge: bool,
) {
    let mut pending = PendingActions::new();
    let mut startup_tx = Some(startup_tx);
    let mut attempt: u32 = 0;

    loop {
        let _ = state_tx.send(ConnectionState::Connecting);
        tracing::info!(address = %address, attempt, "connecting to AMI");

        let exit = match tokio::time::timeout(Duration::from_secs(10), TcpStream::connect(&address))
            .await
        {
            Ok(Ok(stream)) => {
                tracing::info!(address = %address, "TCP connected to AMI");
                run_connected(
                    stream,
                    &credentials,
                    &mut command_rx,
                    &event_bus,
                    &state_tx,
                    &mut startup_tx,
                    &mut pending,
                    &mut attempt,
                    ping_interval,
                    require_challenge,
                )
                .await
            }
            Ok(Err(source)) => {
                let error =
                    AmiError::Connection(asterisk_rs_core::error::ConnectionError::ConnectFailed {
                        address: address.clone(),
                        source,
                    });
                tracing::error!(address = %address, error = %error, "failed to connect to AMI");
                ConnectionExit::Retry(error)
            }
            Err(_) => {
                tracing::error!(address = %address, "AMI connection timed out");
                ConnectionExit::Retry(AmiError::Timeout(
                    asterisk_rs_core::error::TimeoutError::Connection {
                        elapsed: Duration::from_secs(10),
                    },
                ))
            }
        };

        let last_error = match exit {
            ConnectionExit::Shutdown => {
                pending.fail_all_unknown();
                let _ = state_tx.send(ConnectionState::Disconnected);
                return;
            }
            ConnectionExit::Fatal(error) => {
                pending.fail_all_unknown();
                report_startup_error(&mut startup_tx, error);
                let _ = state_tx.send(ConnectionState::Disconnected);
                return;
            }
            ConnectionExit::Retry(error) => {
                pending.fail_all_unknown();
                error
            }
        };

        // reconnection logic
        if reconnect_policy
            .max_retries
            .is_some_and(|max| attempt >= max)
        {
            tracing::error!(error = %last_error, "max reconnection attempts reached, giving up");
            report_startup_error(&mut startup_tx, last_error);
            let _ = state_tx.send(ConnectionState::Disconnected);
            return;
        }

        let _ = state_tx.send(ConnectionState::Reconnecting);
        let delay = reconnect_policy.delay_for_attempt(attempt);
        tracing::info!(?delay, attempt, "reconnecting to AMI");
        // poll shutdown during the reconnect sleep so we exit promptly;
        // drain ALL queued commands so callers fail fast instead of
        // blocking until the backoff timer expires (CONC-001)
        tokio::select! {
            () = tokio::time::sleep(delay) => {
                // backoff complete — drain any commands that arrived
                drain_backoff_commands(&mut command_rx, &state_tx);
            }
            cmd = command_rx.recv() => {
                if reject_backoff_command(cmd, &state_tx) {
                    return; // shutdown requested
                }
                // drain remaining queued commands
                drain_backoff_commands(&mut command_rx, &state_tx);
            }
        }
        attempt += 1;
    }
}

enum ConnectionExit {
    Retry(AmiError),
    Fatal(AmiError),
    Shutdown,
}

struct PendingPing {
    action_id: String,
    deadline: Instant,
}

#[allow(clippy::too_many_arguments)]
async fn run_connected(
    stream: TcpStream,
    credentials: &Credentials,
    command_rx: &mut mpsc::Receiver<ConnectionCommand>,
    event_bus: &EventBus<AmiEvent>,
    state_tx: &watch::Sender<ConnectionState>,
    startup_tx: &mut Option<oneshot::Sender<Result<()>>>,
    pending: &mut PendingActions,
    attempt: &mut u32,
    ping_interval: Option<Duration>,
    require_challenge: bool,
) -> ConnectionExit {
    let (read_half, write_half) = stream.into_split();
    let mut reader = FramedRead::new(read_half, AmiCodec::new());
    let mut writer = FramedWrite::new(write_half, AmiCodec::new());

    let login_result = tokio::time::timeout(
        Duration::from_secs(30),
        perform_login(credentials, &mut reader, &mut writer, require_challenge),
    )
    .await;
    match login_result {
        Ok(Ok(())) => {}
        Ok(Err(error @ AmiError::Auth(_))) => {
            tracing::error!(error = %error, "AMI authentication rejected");
            return ConnectionExit::Fatal(error);
        }
        Ok(Err(error)) => {
            tracing::error!(error = %error, "AMI login exchange failed");
            return ConnectionExit::Retry(error);
        }
        Err(_) => {
            tracing::error!("AMI login timed out after 30s");
            return ConnectionExit::Retry(AmiError::Timeout(
                asterisk_rs_core::error::TimeoutError::Connection {
                    elapsed: Duration::from_secs(30),
                },
            ));
        }
    }

    tracing::info!("AMI login successful");
    *attempt = 0;
    let _ = state_tx.send(ConnectionState::Connected);
    if let Some(tx) = startup_tx.take() {
        if tx.send(Ok(())).is_err() {
            return ConnectionExit::Shutdown;
        }
    }

    let mut ping_timer = ping_interval.map(|interval| {
        let mut timer = tokio::time::interval(interval);
        timer.set_missed_tick_behavior(MissedTickBehavior::Skip);
        timer
    });
    if let Some(timer) = ping_timer.as_mut() {
        timer.tick().await;
    }
    let mut pending_ping: Option<PendingPing> = None;

    loop {
        let next_deadline = pending.next_deadline();
        tokio::select! {
            frame = reader.next() => {
                match frame {
                    Some(Ok(raw)) => {
                        dispatch_message(raw, pending, event_bus, &mut pending_ping);
                    }
                    Some(Err(error)) => {
                        tracing::error!(error = %error, "AMI codec error");
                        return ConnectionExit::Retry(error);
                    }
                    None => {
                        tracing::warn!("AMI connection closed");
                        return ConnectionExit::Retry(AmiError::Disconnected);
                    }
                }
            }
            cmd = command_rx.recv() => {
                let Some(cmd) = cmd else {
                    return ConnectionExit::Shutdown;
                };
                pending.purge_closed();
                match cmd {
                    ConnectionCommand::SendAction {
                        message,
                        action_id,
                        deadline,
                        timeout,
                        lifecycle,
                        response_tx,
                    } => {
                        if response_tx.is_closed() {
                            continue;
                        }
                        if let Err(error) = AmiCodec::validate_outbound(&message) {
                            fail_unsent(lifecycle, response_tx, error);
                            continue;
                        }
                        if pending.at_in_flight_limit() {
                            fail_unsent(
                                lifecycle,
                                response_tx,
                                AmiError::InFlightLimitExceeded {
                                    limit: MAX_IN_FLIGHT_ACTIONS,
                                },
                            );
                            continue;
                        }
                        if !start_request(&lifecycle, deadline) {
                            continue;
                        }
                        match tokio::time::timeout_at(deadline, writer.send(message)).await {
                            Ok(Ok(())) => {
                                lifecycle.mark_sent();
                                pending.register_managed(
                                    action_id,
                                    deadline,
                                    lifecycle,
                                    response_tx,
                                );
                            }
                            Ok(Err(error)) => {
                                fail_unknown(action_id, lifecycle, response_tx);
                                tracing::error!(error = %error, "failed to send AMI action");
                                return ConnectionExit::Retry(error);
                            }
                            Err(_) => {
                                fail_unknown(action_id, lifecycle, response_tx);
                                tracing::warn!(?timeout, "AMI action write exceeded its deadline");
                                return ConnectionExit::Retry(AmiError::Timeout(
                                    asterisk_rs_core::error::TimeoutError::Action { elapsed: timeout },
                                ));
                            }
                        }
                    }
                    ConnectionCommand::SendEventGeneratingAction {
                        message,
                        action_id,
                        deadline,
                        timeout,
                        lifecycle,
                        response_tx,
                    } => {
                        if response_tx.is_closed() {
                            continue;
                        }
                        if let Err(error) = AmiCodec::validate_outbound(&message) {
                            fail_unsent(lifecycle, response_tx, error);
                            continue;
                        }
                        if pending.at_in_flight_limit() {
                            fail_unsent(
                                lifecycle,
                                response_tx,
                                AmiError::InFlightLimitExceeded {
                                    limit: MAX_IN_FLIGHT_ACTIONS,
                                },
                            );
                            continue;
                        }
                        if pending.at_event_list_limit() {
                            fail_unsent(
                                lifecycle,
                                response_tx,
                                AmiError::EventListInFlightLimitExceeded {
                                    limit: MAX_IN_FLIGHT_EVENT_LISTS,
                                },
                            );
                            continue;
                        }
                        if !start_request(&lifecycle, deadline) {
                            continue;
                        }
                        match tokio::time::timeout_at(deadline, writer.send(message)).await {
                            Ok(Ok(())) => {
                                lifecycle.mark_sent();
                                pending.register_managed_event_list(
                                    action_id,
                                    deadline,
                                    lifecycle,
                                    response_tx,
                                );
                            }
                            Ok(Err(error)) => {
                                fail_unknown(action_id, lifecycle, response_tx);
                                tracing::error!(error = %error, "failed to send AMI event-list action");
                                return ConnectionExit::Retry(error);
                            }
                            Err(_) => {
                                fail_unknown(action_id, lifecycle, response_tx);
                                tracing::warn!(?timeout, "AMI event-list write exceeded its deadline");
                                return ConnectionExit::Retry(AmiError::Timeout(
                                    asterisk_rs_core::error::TimeoutError::Action { elapsed: timeout },
                                ));
                            }
                        }
                    }
                    ConnectionCommand::Shutdown => {
                        tracing::info!("AMI connection shutdown requested");
                        return ConnectionExit::Shutdown;
                    }
                }
            }
            _ = wait_for_deadline(next_deadline) => {
                pending.expire(Instant::now());
            }
            _ = wait_for_ping_deadline(pending_ping.as_ref().map(|ping| ping.deadline)) => {
                tracing::warn!("keep-alive pong deadline elapsed, treating connection as dead");
                return ConnectionExit::Retry(AmiError::Disconnected);
            }
            _ = wait_for_ping(&mut ping_timer) => {
                if pending_ping.is_some() {
                    continue;
                }
                let (action_id, ping_message) = PingAction.to_message();
                let Some(write_timeout) = ping_interval else {
                    continue;
                };
                match tokio::time::timeout(write_timeout, writer.send(ping_message)).await {
                    Ok(Ok(())) => {}
                    Ok(Err(error)) => {
                        tracing::warn!(error = %error, "keep-alive ping failed, reconnecting");
                        return ConnectionExit::Retry(error);
                    }
                    Err(_) => {
                        tracing::warn!("keep-alive ping write timed out, reconnecting");
                        return ConnectionExit::Retry(AmiError::Timeout(
                            asterisk_rs_core::error::TimeoutError::Action {
                                elapsed: write_timeout,
                            },
                        ));
                    }
                }
                let Some(deadline) = Instant::now().checked_add(write_timeout) else {
                    return ConnectionExit::Fatal(AmiError::InvalidConfig {
                        details: "ping_interval is too large".to_owned(),
                    });
                };
                pending_ping = Some(PendingPing { action_id, deadline });
                tracing::trace!("keep-alive ping sent");
            }
        }
    }
}

/// perform the AMI login sequence over the raw framed connection
///
/// tries MD5 challenge-response first.  when `require_challenge` is
/// false, falls back to plaintext login (only safe over trusted
/// loopback connections).
async fn perform_login(
    credentials: &Credentials,
    reader: &mut FramedRead<tokio::net::tcp::OwnedReadHalf, AmiCodec>,
    writer: &mut FramedWrite<tokio::net::tcp::OwnedWriteHalf, AmiCodec>,
    require_challenge: bool,
) -> Result<()> {
    // try MD5 challenge-response first
    let (_, challenge_msg) = ChallengeAction.to_message();
    writer.send(challenge_msg).await?;

    let challenge_resp = read_next_response(reader).await?;

    if challenge_resp.success {
        if let Some(challenge) = challenge_resp.get("Challenge") {
            let key = Zeroizing::new(compute_md5_key(challenge, credentials.secret()));
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

    // challenge auth did not produce a Challenge field
    if require_challenge {
        return Err(AmiError::Auth(
            asterisk_rs_core::error::AuthError::Rejected {
                reason: "server did not provide MD5 challenge; plaintext fallback is disabled \
                         (set require_challenge(false) for trusted loopback connections)"
                    .to_owned(),
            },
        ));
    }

    // fall back to plaintext
    tracing::warn!("MD5 challenge auth unavailable, falling back to plaintext login");
    let login = LoginAction::new(credentials.username(), credentials.secret());
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
    let digest = hasher.finalize();
    let mut hex = String::with_capacity(digest.len() * 2);
    for byte in digest {
        use std::fmt::Write;
        let _ = write!(hex, "{byte:02x}");
    }
    hex
}

fn dispatch_message(
    raw: RawAmiMessage,
    pending: &mut PendingActions,
    event_bus: &EventBus<AmiEvent>,
    pending_ping: &mut Option<PendingPing>,
) {
    pending.purge_closed();
    let retained_size = raw.retained_size();

    // try as response first
    if let Some(response) = AmiResponse::from_raw(&raw) {
        if pending_ping
            .as_ref()
            .is_some_and(|ping| ping.action_id == response.action_id)
        {
            if response.success && response.get("Ping") == Some("Pong") {
                pending_ping.take();
                tracing::trace!("keep-alive pong received");
            } else {
                tracing::warn!(
                    action_id = response.action_id,
                    "keep-alive response did not contain the expected pong"
                );
            }
            return;
        }

        if pending.contains_event_list(&response.action_id) {
            pending.deliver_event_list_response_with_size(response, retained_size);
            return;
        }

        // regular action response
        let action_id = response.action_id.clone();
        if !pending.deliver(response) {
            tracing::debug!(action_id, "received response for unknown action");
        }
        return;
    }

    // try as event
    if let Some(event) = AmiEvent::from_raw(&raw) {
        // check if event has an ActionID matching a pending event list
        if let Some(aid) = raw.get("ActionID") {
            let terminal = match raw.get("EventList") {
                Some(value) if value.eq_ignore_ascii_case("Complete") => {
                    EventListTerminal::Complete
                }
                Some(value) if value.eq_ignore_ascii_case("Cancelled") => {
                    EventListTerminal::Cancelled
                }
                _ if event.is_event_list_complete() => EventListTerminal::Complete,
                _ => EventListTerminal::Continue,
            };
            if pending.deliver_event_list_event_with_metadata(
                aid,
                event.clone(),
                retained_size,
                terminal,
            ) {
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

fn start_request(lifecycle: &RequestLifecycle, deadline: Instant) -> bool {
    if Instant::now() >= deadline {
        lifecycle.cancel_queued();
        return false;
    }
    lifecycle.begin_write()
}

fn fail_unknown<T>(
    action_id: String,
    lifecycle: Arc<RequestLifecycle>,
    response_tx: oneshot::Sender<Result<T>>,
) {
    let _ = response_tx.send(Err(AmiError::OutcomeUnknown { action_id }));
    lifecycle.mark_completed();
}

fn fail_unsent<T>(
    lifecycle: Arc<RequestLifecycle>,
    response_tx: oneshot::Sender<Result<T>>,
    error: AmiError,
) {
    lifecycle.cancel_queued();
    let _ = response_tx.send(Err(error));
}

async fn wait_for_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

async fn wait_for_ping(timer: &mut Option<tokio::time::Interval>) {
    match timer {
        Some(timer) => {
            timer.tick().await;
        }
        None => std::future::pending().await,
    }
}

async fn wait_for_ping_deadline(deadline: Option<Instant>) {
    match deadline {
        Some(deadline) => tokio::time::sleep_until(deadline).await,
        None => std::future::pending().await,
    }
}

fn report_startup_error(startup_tx: &mut Option<oneshot::Sender<Result<()>>>, error: AmiError) {
    if let Some(tx) = startup_tx.take() {
        let _ = tx.send(Err(error));
    }
}

/// reject a single command received during reconnect backoff.
/// returns true if the connection task should shut down.
fn reject_backoff_command(
    cmd: Option<ConnectionCommand>,
    state_tx: &watch::Sender<ConnectionState>,
) -> bool {
    match cmd {
        None | Some(ConnectionCommand::Shutdown) => {
            tracing::info!("shutdown received during reconnect backoff");
            let _ = state_tx.send(ConnectionState::Disconnected);
            true
        }
        Some(ConnectionCommand::SendAction {
            lifecycle,
            response_tx,
            ..
        }) => {
            // drop the sender so the caller's oneshot receiver resolves
            // immediately with RecvError instead of waiting for timeout
            tracing::debug!("rejecting action received during reconnect backoff");
            lifecycle.cancel_queued();
            let _ = response_tx.send(Err(AmiError::Disconnected));
            false
        }
        Some(ConnectionCommand::SendEventGeneratingAction {
            lifecycle,
            response_tx,
            ..
        }) => {
            tracing::debug!("rejecting event-list action received during reconnect backoff");
            lifecycle.cancel_queued();
            let _ = response_tx.send(Err(AmiError::Disconnected));
            false
        }
    }
}

/// drain all queued commands during reconnect backoff so callers
/// fail fast. does NOT block — only processes already-queued commands.
fn drain_backoff_commands(
    command_rx: &mut mpsc::Receiver<ConnectionCommand>,
    state_tx: &watch::Sender<ConnectionState>,
) {
    while let Ok(cmd) = command_rx.try_recv() {
        if reject_backoff_command(Some(cmd), state_tx) {
            return;
        }
    }
}
