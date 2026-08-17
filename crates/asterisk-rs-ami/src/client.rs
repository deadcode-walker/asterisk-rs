//! AMI client with builder pattern.

use crate::action::{self, AmiAction, LogoffAction, PingAction};
use crate::connection::{ConnectionCommand, ConnectionManager};
use crate::error::{AmiError, AmiTerminalError, Result};
use crate::event::AmiEvent;
use crate::response::{AmiResponse, RequestLifecycle};
use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::event::{EventBus, EventSubscription};

use std::sync::Arc;
use std::time::Duration;
use tokio::time::Instant;

/// default AMI port
const DEFAULT_PORT: u16 = 5038;
/// default action timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);
/// default initial connection and authentication timeout
const DEFAULT_CONNECT_TIMEOUT: Duration = Duration::from_secs(30);

/// async client for the Asterisk Manager Interface
#[derive(Clone)]
pub struct AmiClient {
    connection: Arc<ConnectionManager>,
    event_bus: EventBus<AmiEvent>,
    credentials: Credentials,
    timeout: Duration,
}

impl AmiClient {
    /// create a new builder
    pub fn builder() -> AmiClientBuilder {
        AmiClientBuilder::default()
    }

    /// send a typed action and wait for the response
    pub async fn send_action<A: AmiAction>(&self, action: &A) -> Result<AmiResponse> {
        let (action_id, message) = action.to_message();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let lifecycle = Arc::new(RequestLifecycle::default());
        let deadline = self.action_deadline()?;

        self.connection
            .send(
                ConnectionCommand::SendAction {
                    message,
                    action_id: action_id.clone(),
                    deadline,
                    timeout: self.timeout,
                    lifecycle: lifecycle.clone(),
                    response_tx,
                },
                deadline,
                self.timeout,
            )
            .await?;

        self.await_action_result(action_id, deadline, lifecycle, response_rx)
            .await
    }

    /// send a ping (keep-alive)
    pub async fn ping(&self) -> Result<AmiResponse> {
        self.send_action(&PingAction).await
    }

    /// originate a call
    pub async fn originate(&self, action: action::OriginateAction) -> Result<AmiResponse> {
        self.send_action(&action).await
    }

    /// hangup a channel
    pub async fn hangup(&self, action: action::HangupAction) -> Result<AmiResponse> {
        self.send_action(&action).await
    }

    /// execute a CLI command
    pub async fn command(&self, command: impl Into<String>) -> Result<AmiResponse> {
        self.send_action(&action::CommandAction::new(command)).await
    }

    /// subscribe to all AMI events
    pub fn subscribe(&self) -> EventSubscription<AmiEvent> {
        self.event_bus.subscribe()
    }

    /// create a call tracker that correlates events into call lifecycle objects
    pub fn call_tracker(
        &self,
    ) -> (
        crate::tracker::CallTracker,
        tokio::sync::mpsc::Receiver<crate::tracker::CompletedCall>,
    ) {
        crate::tracker::CallTracker::new(self.subscribe())
    }

    /// send an action that returns its results as a list of events
    ///
    /// actions like `Status`, `CoreShowChannels`, `QueueStatus`, etc.
    /// return a series of events terminated by a `*Complete` event.
    /// this method collects all events and returns them as a single response.
    pub async fn send_collecting<A: AmiAction>(
        &self,
        action: &A,
    ) -> Result<crate::response::EventListResponse> {
        let (action_id, message) = action.to_message();
        let (response_tx, response_rx) = tokio::sync::oneshot::channel();
        let lifecycle = Arc::new(RequestLifecycle::default());
        let deadline = self.action_deadline()?;

        self.connection
            .send(
                ConnectionCommand::SendEventGeneratingAction {
                    message,
                    action_id: action_id.clone(),
                    deadline,
                    timeout: self.timeout,
                    lifecycle: lifecycle.clone(),
                    response_tx,
                },
                deadline,
                self.timeout,
            )
            .await?;

        self.await_action_result(action_id, deadline, lifecycle, response_rx)
            .await
    }

    /// subscribe to events matching a filter predicate
    ///
    /// ```rust,ignore
    /// // subscribe only to hangup events
    /// let mut hangups = client.subscribe_filtered(|e| {
    ///     e.event_name() == "Hangup"
    /// });
    /// ```
    pub fn subscribe_filtered(
        &self,
        predicate: impl Fn(&AmiEvent) -> bool + Send + 'static,
    ) -> asterisk_rs_core::event::FilteredSubscription<AmiEvent> {
        self.event_bus.subscribe_filtered(predicate)
    }

    /// get current connection state
    pub fn connection_state(&self) -> ConnectionState {
        self.connection.state()
    }

    /// Terminal connection cause retained after startup or reconnect exhaustion.
    pub fn terminal_error(&self) -> Option<AmiTerminalError> {
        self.connection.terminal_error()
    }

    /// gracefully disconnect
    pub async fn disconnect(&self) -> Result<()> {
        // best-effort logoff before closing the connection
        let _ = self.send_action(&LogoffAction).await;
        self.connection.shutdown().await;
        Ok(())
    }

    fn action_deadline(&self) -> Result<Instant> {
        Instant::now()
            .checked_add(self.timeout)
            .ok_or_else(|| AmiError::InvalidConfig {
                details: "action timeout is too large".to_owned(),
            })
    }

    async fn await_action_result<T>(
        &self,
        action_id: String,
        deadline: Instant,
        lifecycle: Arc<RequestLifecycle>,
        mut response_rx: tokio::sync::oneshot::Receiver<Result<T>>,
    ) -> Result<T> {
        match tokio::time::timeout_at(deadline, &mut response_rx).await {
            Ok(Ok(result)) => result,
            Ok(Err(_)) => Err(AmiError::ResponseChannelClosed),
            Err(_) => {
                if let Ok(result) = response_rx.try_recv() {
                    return result;
                }
                if lifecycle.cancel_queued() {
                    Err(self.action_timeout())
                } else if lifecycle.may_have_executed() {
                    Err(AmiError::OutcomeUnknown { action_id })
                } else {
                    Err(self.action_timeout())
                }
            }
        }
    }

    fn action_timeout(&self) -> AmiError {
        AmiError::Timeout(asterisk_rs_core::error::TimeoutError::Action {
            elapsed: self.timeout,
        })
    }
}

impl std::fmt::Debug for AmiClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AmiClient")
            .field("state", &self.connection.state())
            .field("credentials", &self.credentials)
            .finish()
    }
}

/// builder for [`AmiClient`]
#[derive(Debug)]
#[must_use]
pub struct AmiClientBuilder {
    host: String,
    port: u16,
    credentials: Option<Credentials>,
    reconnect_policy: ReconnectPolicy,
    timeout: Duration,
    connect_timeout: Duration,
    event_capacity: usize,
    ping_interval: Option<Duration>,
    require_challenge: bool,
}

impl Default for AmiClientBuilder {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            credentials: None,
            reconnect_policy: ReconnectPolicy::default(),
            timeout: DEFAULT_TIMEOUT,
            connect_timeout: DEFAULT_CONNECT_TIMEOUT,
            event_capacity: 1024,
            ping_interval: None,
            require_challenge: true,
        }
    }
}

impl AmiClientBuilder {
    /// set the asterisk host (default `127.0.0.1`)
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// set the ami port (default 5038)
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// set the ami login credentials
    pub fn credentials(mut self, username: impl Into<String>, secret: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::new(username, secret));
        self
    }

    /// set the reconnect policy
    pub fn reconnect(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    /// set the action response timeout (default 30s)
    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    /// set the deadline for the initial TCP connection and authentication
    pub fn connect_timeout(mut self, timeout: Duration) -> Self {
        self.connect_timeout = timeout;
        self
    }

    /// set the event channel buffer capacity (default 1024)
    pub fn event_capacity(mut self, capacity: usize) -> Self {
        self.event_capacity = capacity;
        self
    }

    /// require MD5 challenge authentication instead of plaintext login fallback
    ///
    /// When `true`, login fails if challenge-response is unavailable. The
    /// default is `false`; use plaintext fallback only inside a separately
    /// authenticated TLS boundary because native AMI transport is TCP.
    pub fn require_challenge(mut self, require: bool) -> Self {
        self.require_challenge = require;
        self
    }

    /// set the interval for keep-alive pings
    ///
    /// when set, the client sends periodic Ping actions to detect
    /// dead connections. a reasonable default is 20 seconds.
    /// disabled by default.
    pub fn ping_interval(mut self, interval: Duration) -> Self {
        self.ping_interval = Some(interval);
        self
    }

    /// build and connect the client
    ///
    /// waits for TCP connection and login before returning
    pub async fn build(self) -> Result<AmiClient> {
        let credentials = self.credentials.ok_or(AmiError::Auth(
            asterisk_rs_core::error::AuthError::InvalidCredentials,
        ))?;

        if self.event_capacity == 0 {
            return Err(AmiError::InvalidConfig {
                details: "event_capacity must be greater than zero".to_owned(),
            });
        }
        if self.timeout.is_zero() {
            return Err(AmiError::InvalidConfig {
                details: "timeout must be greater than zero".to_owned(),
            });
        }
        if self.connect_timeout.is_zero() {
            return Err(AmiError::InvalidConfig {
                details: "connect_timeout must be greater than zero".to_owned(),
            });
        }
        if self.ping_interval == Some(Duration::ZERO) {
            return Err(AmiError::InvalidConfig {
                details: "ping_interval must be greater than zero".to_owned(),
            });
        }
        if self
            .ping_interval
            .is_some_and(|interval| Instant::now().checked_add(interval).is_none())
        {
            return Err(AmiError::InvalidConfig {
                details: "ping_interval is too large".to_owned(),
            });
        }
        if let Err(details) = self.reconnect_policy.validate() {
            return Err(AmiError::InvalidConfig {
                details: details.to_owned(),
            });
        }

        let event_bus = EventBus::new(self.event_capacity);
        let address = format!("{}:{}", self.host, self.port);

        let (connection, startup_rx) = ConnectionManager::spawn(
            address,
            credentials.clone(),
            event_bus.clone(),
            self.reconnect_policy,
            self.ping_interval,
            self.require_challenge,
        );

        // bound the complete initial connect/retry/authentication sequence
        match tokio::time::timeout(self.connect_timeout, startup_rx).await {
            Ok(Ok(result)) => result?,
            Ok(Err(_)) => return Err(AmiError::Disconnected),
            Err(_) => {
                return Err(AmiError::Timeout(
                    asterisk_rs_core::error::TimeoutError::Connection {
                        elapsed: self.connect_timeout,
                    },
                ));
            }
        }

        Ok(AmiClient {
            connection: Arc::new(connection),
            event_bus,
            credentials,
            timeout: self.timeout,
        })
    }
}
