//! ARI client configuration and builder.

use asterisk_rs_core::config::ReconnectPolicy;
use url::Url;

use crate::error::{AriError, Result};

/// ARI connection configuration
#[derive(Debug, Clone)]
pub struct AriConfig {
    pub base_url: Url,
    pub username: String,
    pub password: String,
    pub app_name: String,
    pub ws_url: Url,
    pub reconnect_policy: ReconnectPolicy,
}

/// builder for constructing an [`AriConfig`] with validation
pub struct AriConfigBuilder {
    host: String,
    port: u16,
    username: String,
    password: String,
    app_name: String,
    secure: bool,
    reconnect_policy: ReconnectPolicy,
}

impl AriConfigBuilder {
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 8088,
            username: String::new(),
            password: String::new(),
            app_name: app_name.into(),
            secure: false,
            reconnect_policy: ReconnectPolicy::default(),
        }
    }

    pub fn host(mut self, host: impl Into<String>) -> Self {
        self.host = host.into();
        self
    }

    pub fn port(mut self, port: u16) -> Self {
        self.port = port;
        self
    }

    pub fn username(mut self, username: impl Into<String>) -> Self {
        self.username = username.into();
        self
    }

    pub fn password(mut self, password: impl Into<String>) -> Self {
        self.password = password.into();
        self
    }

    pub fn app_name(mut self, app_name: impl Into<String>) -> Self {
        self.app_name = app_name.into();
        self
    }

    pub fn secure(mut self, secure: bool) -> Self {
        self.secure = secure;
        self
    }

    pub fn reconnect(mut self, policy: ReconnectPolicy) -> Self {
        self.reconnect_policy = policy;
        self
    }

    /// build the config, constructing base and websocket URLs
    ///
    /// fails if app_name is empty or URLs cannot be parsed
    pub fn build(self) -> Result<AriConfig> {
        if self.app_name.is_empty() {
            return Err(AriError::InvalidUrl("app_name must not be empty".to_owned()));
        }

        let http_scheme = if self.secure { "https" } else { "http" };
        let ws_scheme = if self.secure { "wss" } else { "ws" };

        let base_url_str = format!("{http_scheme}://{}:{}/ari", self.host, self.port);
        let base_url = Url::parse(&base_url_str)
            .map_err(|e| AriError::InvalidUrl(e.to_string()))?;

        // ws url includes api_key for authentication
        let ws_url_str = format!(
            "{ws_scheme}://{}:{}/ari/events?app={}&api_key={}:{}",
            self.host, self.port, self.app_name, self.username, self.password,
        );
        let ws_url = Url::parse(&ws_url_str)
            .map_err(|e| AriError::InvalidUrl(e.to_string()))?;

        Ok(AriConfig {
            base_url,
            username: self.username,
            password: self.password,
            app_name: self.app_name,
            ws_url,
            reconnect_policy: self.reconnect_policy,
        })
    }
}
