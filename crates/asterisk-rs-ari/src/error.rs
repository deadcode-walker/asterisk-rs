//! ARI-specific error types.

/// errors that can occur during ARI operations
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AriError {
    #[error("HTTP request failed: {0}")]
    Http(#[from] reqwest::Error),

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
}

pub type Result<T> = std::result::Result<T, AriError>;
