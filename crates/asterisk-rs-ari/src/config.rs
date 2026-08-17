//! ARI client configuration and builder.

use std::time::Duration;

use asterisk_rs_core::auth::Credentials;
use asterisk_rs_core::config::ReconnectPolicy;
use url::Url;
use zeroize::Zeroizing;

use rustls::pki_types::pem::PemObject;

use crate::error::{AriError, Result};

/// default deadline for one ARI REST operation
pub const DEFAULT_REQUEST_TIMEOUT: Duration = Duration::from_secs(30);
/// default maximum buffered HTTP response body
pub const DEFAULT_MAX_RESPONSE_BODY_BYTES: usize = 4 * 1024 * 1024;
/// default maximum WebSocket message and frame size
pub const DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Clone, Default)]
pub(crate) struct TlsTrust {
    pub(crate) reqwest_roots: Vec<reqwest::Certificate>,
    pub(crate) rustls_roots: Vec<rustls::pki_types::CertificateDer<'static>>,
}

pub(crate) fn parse_private_ca_pem(pem: &[u8]) -> Result<TlsTrust> {
    let reqwest_roots = reqwest::Certificate::from_pem_bundle(pem)
        .map_err(|error| AriError::InvalidConfig(format!("invalid private CA PEM: {error}")))?;
    let rustls_roots = rustls::pki_types::CertificateDer::pem_slice_iter(pem)
        .collect::<std::result::Result<Vec<_>, _>>()
        .map_err(|error| AriError::InvalidConfig(format!("invalid private CA PEM: {error}")))?;
    if reqwest_roots.is_empty() || rustls_roots.is_empty() {
        return Err(AriError::InvalidConfig(
            "private CA PEM contains no certificates".to_owned(),
        ));
    }
    Ok(TlsTrust {
        reqwest_roots,
        rustls_roots,
    })
}

pub(crate) fn is_loopback_host(host: &str) -> bool {
    host.eq_ignore_ascii_case("localhost")
        || host
            .parse::<std::net::IpAddr>()
            .is_ok_and(|address| address.is_loopback())
}

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
    /// policy controlling reconnect behavior
    pub(crate) reconnect_policy: ReconnectPolicy,
    /// transport mode for rest communication
    pub(crate) transport_mode: TransportMode,
    /// deadline for one REST operation, including queue admission
    pub(crate) request_timeout: Duration,
    /// maximum HTTP response body buffered by the client
    pub(crate) max_response_body_bytes: usize,
    /// maximum inbound WebSocket message and frame size
    pub(crate) max_websocket_message_bytes: usize,
    pub(crate) tls_trust: TlsTrust,
}

impl std::fmt::Debug for AriConfig {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriConfig")
            .field("base_url", &self.base_url)
            .field("credentials", &self.credentials)
            .field("app_name", &self.app_name)
            .field("reconnect_policy", &self.reconnect_policy)
            .field("transport_mode", &self.transport_mode)
            .field("request_timeout", &self.request_timeout)
            .field("max_response_body_bytes", &self.max_response_body_bytes)
            .field(
                "max_websocket_message_bytes",
                &self.max_websocket_message_bytes,
            )
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
    pub(crate) fn ws_url(&self) -> Zeroizing<String> {
        let mut url = self.base_url.clone();
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme)
            .expect("http and ws schemes are valid");
        url.set_path("/ari/events");
        url.set_query(None);
        let api_key = Zeroizing::new(format!(
            "{}:{}",
            self.credentials.username(),
            self.credentials.secret()
        ));
        url.query_pairs_mut()
            .append_pair("app", &self.app_name)
            .append_pair("api_key", &api_key);
        Zeroizing::new(url.into())
    }

    /// policy controlling reconnect behavior
    pub fn reconnect_policy(&self) -> &ReconnectPolicy {
        &self.reconnect_policy
    }

    /// transport mode for rest communication
    pub fn transport_mode(&self) -> TransportMode {
        self.transport_mode
    }

    /// deadline for one REST operation, including transport queue admission
    pub fn request_timeout(&self) -> Duration {
        self.request_timeout
    }

    /// maximum HTTP response body buffered in application memory
    pub fn max_response_body_bytes(&self) -> usize {
        self.max_response_body_bytes
    }

    /// maximum inbound WebSocket message and frame size
    pub fn max_websocket_message_bytes(&self) -> usize {
        self.max_websocket_message_bytes
    }
}

/// builder for constructing an [`AriConfig`] with validation
#[must_use]
pub struct AriConfigBuilder {
    host: String,
    port: u16,
    username: String,
    password: Zeroizing<String>,
    app_name: String,
    secure: bool,
    reconnect_policy: ReconnectPolicy,
    transport_mode: TransportMode,
    request_timeout: Duration,
    max_response_body_bytes: usize,
    max_websocket_message_bytes: usize,
    allow_insecure_remote: bool,
    private_ca_pem: Option<Vec<u8>>,
}

impl std::fmt::Debug for AriConfigBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("AriConfigBuilder")
            .field("host", &self.host)
            .field("port", &self.port)
            .field("username", &self.username)
            .field("password", &"[redacted]")
            .field("app_name", &self.app_name)
            .field("secure", &self.secure)
            .field("transport_mode", &self.transport_mode)
            .field("request_timeout", &self.request_timeout)
            .field("max_response_body_bytes", &self.max_response_body_bytes)
            .field(
                "max_websocket_message_bytes",
                &self.max_websocket_message_bytes,
            )
            .finish()
    }
}

impl AriConfigBuilder {
    /// create a builder with the given stasis application name
    pub fn new(app_name: impl Into<String>) -> Self {
        Self {
            host: "127.0.0.1".to_owned(),
            port: 8088,
            username: String::new(),
            password: Zeroizing::new(String::new()),
            app_name: app_name.into(),
            secure: false,
            reconnect_policy: ReconnectPolicy::default(),
            transport_mode: TransportMode::default(),
            request_timeout: DEFAULT_REQUEST_TIMEOUT,
            max_response_body_bytes: DEFAULT_MAX_RESPONSE_BODY_BYTES,
            max_websocket_message_bytes: DEFAULT_MAX_WEBSOCKET_MESSAGE_BYTES,
            allow_insecure_remote: false,
            private_ca_pem: None,
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
        self.password = Zeroizing::new(password.into());
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

    /// explicitly permit cleartext HTTP/WebSocket transport to a non-loopback host
    ///
    /// Remote cleartext exposes ARI credentials and traffic. Prefer [`Self::secure`]
    /// and use this only behind a separately secured, trusted network boundary.
    pub fn allow_insecure_remote(mut self, allow: bool) -> Self {
        self.allow_insecure_remote = allow;
        self
    }

    /// add one or more PEM-encoded private CA certificates for HTTPS and WSS
    ///
    /// The bundle is parsed during [`Self::build`] and augments, rather than
    /// replaces, the platform trust store.
    pub fn private_ca_pem(mut self, pem: impl Into<Vec<u8>>) -> Self {
        self.private_ca_pem = Some(pem.into());
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

    /// set the deadline for one REST operation (default 30 seconds)
    pub fn request_timeout(mut self, timeout: Duration) -> Self {
        self.request_timeout = timeout;
        self
    }

    /// cap buffered HTTP response bodies (default 4 MiB)
    pub fn max_response_body_bytes(mut self, bytes: usize) -> Self {
        self.max_response_body_bytes = bytes;
        self
    }

    /// cap inbound messages/frames and outbound REST envelopes (default 4 MiB)
    pub fn max_websocket_message_bytes(mut self, bytes: usize) -> Self {
        self.max_websocket_message_bytes = bytes;
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
        if self.request_timeout.is_zero() {
            return Err(AriError::InvalidConfig(
                "request_timeout must be greater than zero".to_owned(),
            ));
        }
        if std::time::Instant::now()
            .checked_add(self.request_timeout)
            .is_none()
        {
            return Err(AriError::InvalidConfig(
                "request_timeout is too large for the platform clock".to_owned(),
            ));
        }
        if self.max_response_body_bytes == 0 {
            return Err(AriError::InvalidConfig(
                "max_response_body_bytes must be greater than zero".to_owned(),
            ));
        }
        if self.max_websocket_message_bytes == 0 {
            return Err(AriError::InvalidConfig(
                "max_websocket_message_bytes must be greater than zero".to_owned(),
            ));
        }
        if let Err(details) = self.reconnect_policy.validate() {
            return Err(AriError::InvalidConfig(details.to_owned()));
        }
        if !self.secure && !self.allow_insecure_remote && !is_loopback_host(&self.host) {
            return Err(AriError::InvalidConfig(format!(
                "cleartext ARI transport to non-loopback host '{}' requires allow_insecure_remote(true)",
                self.host
            )));
        }
        let tls_trust = self
            .private_ca_pem
            .as_deref()
            .map(parse_private_ca_pem)
            .transpose()?
            .unwrap_or_default();

        let http_scheme = if self.secure { "https" } else { "http" };
        let url_host = if self.host.parse::<std::net::Ipv6Addr>().is_ok() {
            format!("[{}]", self.host)
        } else {
            self.host.clone()
        };

        let base_url_str = format!("{http_scheme}://{url_host}:{}/ari", self.port);
        let base_url =
            Url::parse(&base_url_str).map_err(|e| AriError::InvalidUrl(e.to_string()))?;

        let credentials = Credentials::new(self.username, &*self.password);

        Ok(AriConfig {
            base_url,
            credentials,
            app_name: self.app_name,
            reconnect_policy: self.reconnect_policy,
            transport_mode: self.transport_mode,
            request_timeout: self.request_timeout,
            max_response_body_bytes: self.max_response_body_bytes,
            max_websocket_message_bytes: self.max_websocket_message_bytes,
            tls_trust,
        })
    }
}
