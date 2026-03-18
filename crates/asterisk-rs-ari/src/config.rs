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
            return Err(AriError::InvalidUrl(
                "app_name must not be empty".to_owned(),
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


#[cfg(test)]
mod tests {
    use super::*;
    use asterisk_rs_core::config::ReconnectPolicy;

    #[test]
    fn build_default_config() {
        let config = AriConfigBuilder::new("myapp")
            .build()
            .expect("default config should build");

        assert_eq!(config.base_url.as_str(), "http://127.0.0.1:8088/ari");
        assert!(
            config.ws_url.as_str().starts_with("ws://"),
            "ws_url should start with ws://, got: {}",
            config.ws_url
        );
    }

    #[test]
    fn build_with_custom_host_port() {
        let config = AriConfigBuilder::new("myapp")
            .host("10.0.0.1")
            .port(9999)
            .build()
            .expect("custom host/port should build");

        assert!(
            config.base_url.as_str().contains("10.0.0.1:9999"),
            "base_url should contain custom host:port, got: {}",
            config.base_url
        );
        assert!(
            config.ws_url.as_str().contains("10.0.0.1:9999"),
            "ws_url should contain custom host:port, got: {}",
            config.ws_url
        );
    }

    #[test]
    fn build_secure_uses_https_wss() {
        let config = AriConfigBuilder::new("myapp")
            .secure(true)
            .build()
            .expect("secure config should build");

        assert!(
            config.base_url.as_str().starts_with("https://"),
            "base_url should use https, got: {}",
            config.base_url
        );
        assert!(
            config.ws_url.as_str().starts_with("wss://"),
            "ws_url should use wss, got: {}",
            config.ws_url
        );
    }

    #[test]
    fn build_empty_app_name_fails() {
        let err = AriConfigBuilder::new("")
            .build()
            .expect_err("empty app_name via constructor should fail");

        match err {
            AriError::InvalidUrl(msg) => {
                assert!(msg.contains("app_name"), "error should mention app_name: {msg}");
            }
            other => panic!("expected InvalidUrl, got: {other:?}"),
        }
    }

    #[test]
    fn build_empty_app_name_via_setter_fails() {
        let err = AriConfigBuilder::new("valid")
            .app_name("")
            .build()
            .expect_err("empty app_name via setter should fail");

        match err {
            AriError::InvalidUrl(msg) => {
                assert!(msg.contains("app_name"), "error should mention app_name: {msg}");
            }
            other => panic!("expected InvalidUrl, got: {other:?}"),
        }
    }

    #[test]
    fn ws_url_contains_app_name() {
        let config = AriConfigBuilder::new("test_app")
            .build()
            .expect("config should build");

        assert!(
            config.ws_url.as_str().contains("app=test_app"),
            "ws_url should contain app=test_app, got: {}",
            config.ws_url
        );
    }

    #[test]
    fn ws_url_contains_credentials() {
        let config = AriConfigBuilder::new("myapp")
            .username("admin")
            .password("secret")
            .build()
            .expect("config with credentials should build");

        assert!(
            config.ws_url.as_str().contains("api_key=admin:secret"),
            "ws_url should contain api_key=admin:secret, got: {}",
            config.ws_url
        );
    }

    #[test]
    fn build_with_custom_reconnect_policy() {
        use std::time::Duration;

        let policy = ReconnectPolicy::fixed(Duration::from_secs(5));

        let config = AriConfigBuilder::new("myapp")
            .reconnect(policy)
            .build()
            .expect("config with reconnect policy should build");

        assert_eq!(config.reconnect_policy.initial_delay, Duration::from_secs(5));
        assert_eq!(config.reconnect_policy.max_delay, Duration::from_secs(5));
    }

    #[test]
    fn config_fields_accessible() {
        let config = AriConfigBuilder::new("myapp")
            .host("asterisk.local")
            .port(5080)
            .username("user1")
            .password("pass1")
            .secure(true)
            .build()
            .expect("full config should build");

        assert_eq!(config.app_name, "myapp");
        assert_eq!(config.username, "user1");
        assert_eq!(config.password, "pass1");
        assert_eq!(config.base_url.as_str(), "https://asterisk.local:5080/ari");
        assert!(config.ws_url.as_str().starts_with("wss://"));
        // reconnect_policy is accessible (default)
        let _ = &config.reconnect_policy;
    }

    #[test]
    fn builder_fluent_chain() {
        // all builder methods return Self, so they can be chained in a single expression
        let result = AriConfigBuilder::new("chain")
            .host("localhost")
            .port(8088)
            .username("u")
            .password("p")
            .app_name("chain2")
            .secure(false)
            .reconnect(ReconnectPolicy::default())
            .build();

        assert!(result.is_ok(), "fluent chain should produce valid config");
    }

    #[test]
    fn default_host_is_localhost() {
        let config = AriConfigBuilder::new("myapp")
            .build()
            .expect("default config should build");

        assert!(
            config.base_url.as_str().contains("127.0.0.1"),
            "default host should be 127.0.0.1, got: {}",
            config.base_url
        );
    }

    #[test]
    fn default_port_is_8088() {
        let config = AriConfigBuilder::new("myapp")
            .build()
            .expect("default config should build");

        assert!(
            config.base_url.as_str().contains(":8088"),
            "default port should be 8088, got: {}",
            config.base_url
        );
    }
}