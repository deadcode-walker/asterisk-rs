//! ARI client configuration and builder.

use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::ReconnectPolicy;
use url::Url;

use crate::error::{AriError, Result};

/// transport mode for ARI client communication
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
#[non_exhaustive]
pub enum TransportMode {
    /// separate HTTP for REST + WebSocket for events (default)
    #[default]
    Http,
    /// unified WebSocket for both REST and events
    ///
    /// requires Asterisk 20.14.0+ / 21.9.0+ / 22.4.0+
    WebSocket,
}

/// ARI connection configuration
#[derive(Clone)]
pub struct AriConfig {
    /// http base url for rest requests
    pub(crate) base_url: Url,
    /// ari credentials
    pub(crate) credentials: Credentials,
    /// stasis application name
    pub(crate) app_name: String,
    /// websocket url for event subscription
    pub(crate) ws_url: Url,
    /// policy controlling reconnect behavior
    pub(crate) reconnect_policy: ReconnectPolicy,
    /// transport mode for rest communication
    pub(crate) transport_mode: TransportMode,
}

impl std::fmt::Debug for AriConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriConfig")
            .field("base_url", &self.base_url)
            .field("credentials", &self.credentials)
            .field("app_name", &self.app_name)
            .field("ws_url", &"[redacted]")
            .field("reconnect_policy", &self.reconnect_policy)
            .field("transport_mode", &self.transport_mode)
            .finish()
    }
}

impl AriConfig {
    /// http base url for rest requests
    pub fn base_url(&self) -> &Url {
        &self.base_url
    }

    /// ari credentials
    pub fn credentials(&self) -> &Credentials {
        &self.credentials
    }

    /// stasis application name
    pub fn app_name(&self) -> &str {
        &self.app_name
    }

    /// websocket url for event subscription (internal only — contains credentials)
    pub(crate) fn ws_url(&self) -> &Url {
        &self.ws_url
    }

    /// policy controlling reconnect behavior
    pub fn reconnect_policy(&self) -> &ReconnectPolicy {
        &self.reconnect_policy
    }

    /// transport mode for rest communication
    pub fn transport_mode(&self) -> TransportMode {
        self.transport_mode
    }
}

/// builder for constructing an [`AriConfig`] with validation
#[must_use]
pub struct AriConfigBuilder {
    host: String,
    port: u16,
    username: String,
    password: String,
    app_name: String,
    secure: bool,
    reconnect_policy: ReconnectPolicy,
    transport_mode: TransportMode,
}

impl AriConfigBuilder {
    /// create a builder with the given stasis application name
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 8088,
            username: String::new(),
            password: String::new(),
            app_name: app_name.into(),
            secure: false,
            reconnect_policy: ReconnectPolicy::default(),
            transport_mode: TransportMode::default(),
        }
    }

    /// set the asterisk host (default `127.0.0.1`)
    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    /// set the http/websocket port (default 8088)
    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    /// set the ari username
    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    /// set the ari password
    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    /// set the stasis application name
    pub fn app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    /// use https/wss when true (default false)
    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    /// set the reconnect policy
    pub fn reconnect(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    /// select the transport mode for REST communication
    ///
    /// [`TransportMode::Http`] (default) uses separate HTTP + WebSocket connections.
    /// [`TransportMode::WebSocket`] sends REST requests over the event WebSocket.
    pub fn transport(mut self, mode: TransportMode) -> Self {
        self.transport_mode = mode;
        self
    }

    /// build the config, constructing base and websocket URLs
    ///
    /// fails if app_name, username, or password is empty, or URLs cannot be parsed
    pub fn build(self) -> Result<AriConfig> {
        if self.app_name.is_empty() {
            return Err(AriError::InvalidUrl(
                "app_name must not be empty".to_owned(),
            ));
        }
        if self.username.is_empty() {
            return Err(AriError::InvalidUrl(
                "username must not be empty".to_owned(),
            ));
        }
        if self.password.is_empty() {
            return Err(AriError::InvalidUrl(
                "password must not be empty".to_owned(),
            ));
        }

        let http_scheme = if self.secure { "https" } else { "http" };
        let ws_scheme = if self.secure { "wss" } else { "ws" };

        let base_url_str = format!("{http_scheme}://{}:{}/ari", self.host, self.port);
        let base_url =
            Url::parse(&base_url_str).map_err(|e| AriError::InvalidUrl(e.to_string()))?;

        // ws url includes api_key for authentication
        let ws_url_str = format!(
            "{ws_scheme}://{}:{}/ari/events?app={}&api_key={}:{}",
            self.host, self.port, self.app_name, self.username, self.password,
        );
        let ws_url = Url::parse(&ws_url_str).map_err(|e| AriError::InvalidUrl(e.to_string()))?;

        let credentials = Credentials::new(self.username, self.password);

        Ok(AriConfig {
            base_url,
            credentials,
            app_name: self.app_name,
            ws_url,
            reconnect_policy: self.reconnect_policy,
            transport_mode: self.transport_mode,
        })
    }
}
