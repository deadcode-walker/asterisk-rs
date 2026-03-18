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

#[cfg(test)]
mod tests {
    use super::*;
    use asterisk_rs_core::error::{AuthError, ConnectionError};

    #[test]
    fn api_error_display() {
        let err = AriError::Api {
            status: 404,
            message: "Not Found".to_owned(),
        };
        assert_eq!(err.to_string(), "API error 404: Not Found");
    }

    #[test]
    fn websocket_error_display() {
        let err = AriError::WebSocket("connection reset".to_owned());
        let msg = err.to_string();
        assert!(msg.contains("connection reset"), "got: {msg}");
    }

    #[test]
    fn disconnected_error_display() {
        let err = AriError::Disconnected;
        assert_eq!(err.to_string(), "client is disconnected");
    }

    #[test]
    fn invalid_url_error_display() {
        let err = AriError::InvalidUrl("bad://url".to_owned());
        let msg = err.to_string();
        assert!(msg.contains("bad://url"), "got: {msg}");
    }

    #[test]
    fn json_error_display() {
        let json_err = serde_json::from_str::<serde_json::Value>("not json").unwrap_err();
        let err = AriError::Json(json_err);
        let msg = err.to_string();
        assert!(msg.contains("JSON error"), "got: {msg}");
    }

    #[test]
    fn io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
        let err = AriError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("pipe broke"), "got: {msg}");
    }

    #[test]
    fn connection_error_display() {
        let err = AriError::Connection(ConnectionError::Closed);
        let msg = err.to_string();
        assert!(
            msg.contains("closed unexpectedly"),
            "got: {msg}"
        );
    }

    #[test]
    fn auth_error_display() {
        let err = AriError::Auth(AuthError::InvalidCredentials);
        let msg = err.to_string();
        assert!(msg.contains("invalid credentials"), "got: {msg}");
    }

    #[test]
    fn errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AriError>();
    }
}
