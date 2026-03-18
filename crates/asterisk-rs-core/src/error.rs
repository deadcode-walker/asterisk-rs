//! Base error types for the asterisk-rs ecosystem.

/// top-level error type encompassing all asterisk-rs failures
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    #[error("connection failed: {0}")]
    Connection(#[from] ConnectionError),

    #[error("authentication failed: {0}")]
    Auth(#[from] AuthError),

    #[error("operation timed out: {0}")]
    Timeout(#[from] TimeoutError),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConnectionError {
    #[error("failed to connect to {address}: {source}")]
    ConnectFailed {
        address: String,
        source: std::io::Error,
    },

    #[error("connection closed unexpectedly")]
    Closed,

    #[error("TLS handshake failed: {0}")]
    Tls(String),

    #[error("DNS resolution failed for {host}: {source}")]
    DnsResolution {
        host: String,
        source: std::io::Error,
    },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AuthError {
    #[error("login rejected: {reason}")]
    Rejected { reason: String },

    #[error("invalid credentials")]
    InvalidCredentials,

    #[error("challenge-response failed")]
    ChallengeFailed,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum TimeoutError {
    #[error("action timed out after {elapsed:?}")]
    Action { elapsed: std::time::Duration },

    #[error("connection timed out after {elapsed:?}")]
    Connection { elapsed: std::time::Duration },
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ProtocolError {
    #[error("malformed message: {details}")]
    MalformedMessage { details: String },

    #[error("unexpected response: expected {expected}, got {actual}")]
    UnexpectedResponse { expected: String, actual: String },

    #[error("unsupported protocol version: {version}")]
    UnsupportedVersion { version: String },
}
