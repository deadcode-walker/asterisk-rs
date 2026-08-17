//! ARI-specific error types.

/// Stable public wrapper for HTTP transport failures.
///
/// The concrete HTTP client is an implementation detail, so upgrading it does
/// not change the public [`AriError`] payload type.
#[derive(Debug)]
pub struct HttpError {
    source: reqwest::Error,
}

impl HttpError {
    pub(crate) fn new(source: reqwest::Error) -> Self {
        Self { source }
    }

    /// Whether the HTTP operation exceeded its configured deadline.
    pub fn is_timeout(&self) -> bool {
        self.source.is_timeout()
    }

    /// HTTP status code associated with the failure, when one is available.
    pub fn status_code(&self) -> Option<u16> {
        self.source.status().map(|status| status.as_u16())
    }
}

impl std::fmt::Display for HttpError {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        self.source.fmt(formatter)
    }
}

impl std::error::Error for HttpError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        Some(&self.source)
    }
}

/// errors that can occur during ARI operations
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AriError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] HttpError),

    #[error("WebSocket error: {0}")]
    WebSocket(String),

    #[error("API error {status}: {message}")]
    Api { status: u16, message: String },

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("connection error: {0}")]
    Connection(#[from] asterisk_rs_core::error::ConnectionError),

    #[error("authentication error: {0}")]
    Auth(#[from] asterisk_rs_core::error::AuthError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("client is disconnected")]
    Disconnected,

    #[error("invalid URL: {0}")]
    InvalidUrl(String),

    #[error("invalid configuration: {0}")]
    InvalidConfig(String),

    #[error("ARI session task failed: {details}")]
    SessionTaskFailed { details: String },

    /// the request expired while still queued and was never written
    #[error("{method} {uri} was not sent")]
    RequestNotSent { method: String, uri: String },

    /// a mutating request started its wire write without a definitive response
    #[error(
        "outcome unknown for ARI request {request_id} ({method} {uri}): it may have been executed"
    )]
    OutcomeUnknown {
        request_id: String,
        method: String,
        uri: String,
    },

    #[error("ARI response body exceeded {limit} byte limit after {received} bytes")]
    ResponseTooLarge { limit: usize, received: u64 },
}

pub type Result<T> = std::result::Result<T, AriError>;
