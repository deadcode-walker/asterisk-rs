//! AMI client with builder pattern.

use crate::action::{self, AmiAction, LogoffAction, PingAction};
use crate::connection::{ConnectionCommand, ConnectionManager};
use crate::error::{AmiError, Result};
use crate::event::AmiEvent;
use crate::response::AmiResponse;
use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::{ConnectionState, ReconnectPolicy};
use asterisk_rs_core::event::{EventBus, EventSubscription};

use std::sync::Arc;
use std::time::Duration;

/// default AMI port
const DEFAULT_PORT: u16 = 5038;
/// default action timeout
const DEFAULT_TIMEOUT: Duration = Duration::from_secs(30);

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

        self.connection
            .send(ConnectionCommand::SendAction {
                message,
                action_id: action_id.clone(),
                response_tx,
            })
            .await?;

        let response = tokio::time::timeout(self.timeout, response_rx)
            .await
            .map_err(|_| {
                AmiError::Timeout(asterisk_rs_core::error::TimeoutError::Action {
                    elapsed: self.timeout,
                })
            })?
            .map_err(|_| AmiError::ResponseChannelClosed)?;

        Ok(response)
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

        self.connection
            .send(ConnectionCommand::SendEventGeneratingAction {
                message,
                action_id: action_id.clone(),
                response_tx,
            })
            .await?;

        let result = tokio::time::timeout(self.timeout, response_rx)
            .await
            .map_err(|_| {
                AmiError::Timeout(asterisk_rs_core::error::TimeoutError::Action {
                    elapsed: self.timeout,
                })
            })?
            .map_err(|_| AmiError::ResponseChannelClosed)?;

        Ok(result)
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

    /// gracefully disconnect
    pub async fn disconnect(&self) -> Result<()> {
        // best-effort logoff before closing the connection
        let _ = self.send_action(&LogoffAction).await;
        self.connection.shutdown().await;
        Ok(())
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
#[must_use]
pub struct AmiClientBuilder {
    host: String,
    port: u16,
    credentials: Option<Credentials>,
    reconnect_policy: ReconnectPolicy,
    timeout: Duration,
    event_capacity: usize,
}

impl Default for AmiClientBuilder {
    fn default() -> Self {
        Self {
            host: "127.0.0.1".to_string(),
            port: DEFAULT_PORT,
            credentials: None,
            reconnect_policy: ReconnectPolicy::default(),
            timeout: DEFAULT_TIMEOUT,
            event_capacity: 1024,
        }
    }
}

impl AmiClientBuilder {
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn credentials(mut self, username: impl Into<String>, secret: impl Into<String>) -> Self {
        self.credentials = Some(Credentials::new(username, secret));
        self
    }

    pub fn reconnect(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    pub fn timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }

    pub fn event_capacity(mut self, capacity: usize) -> Self {
        self.event_capacity = capacity;
        self
    }

    /// build and connect the client
    ///
    /// waits for TCP connection and login before returning
    pub async fn build(self) -> Result<AmiClient> {
        let credentials = self.credentials.ok_or(AmiError::Auth(
            asterisk_rs_core::error::AuthError::InvalidCredentials,
        ))?;

        let event_bus = EventBus::new(self.event_capacity);
        let address = format!("{}:{}", self.host, self.port);

        let connection = ConnectionManager::spawn(
            address,
            credentials.clone(),
            event_bus.clone(),
            self.reconnect_policy,
        );

        // wait for connection + login to complete
        connection
            .wait_for_state(ConnectionState::Connected)
            .await?;

        Ok(AmiClient {
            connection: Arc::new(connection),
            event_bus,
            credentials,
            timeout: self.timeout,
        })
    }
}
