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


#[cfg(test)]
mod tests {
    use super::*;
    use std::time::Duration;

    // compile-time assertion that all error types are send + sync
    fn assert_send_sync<T: Send + Sync>() {}

    #[test]
    fn all_errors_are_send_and_sync() {
        assert_send_sync::<Error>();
        assert_send_sync::<ConnectionError>();
        assert_send_sync::<AuthError>();
        assert_send_sync::<TimeoutError>();
        assert_send_sync::<ProtocolError>();
    }

    // --- Error (top-level) Display ---

    #[test]
    fn error_display_connection() {
        let err = Error::Connection(ConnectionError::Closed);
        assert_eq!(err.to_string(), "connection failed: connection closed unexpectedly");
    }

    #[test]
    fn error_display_auth() {
        let err = Error::Auth(AuthError::InvalidCredentials);
        assert_eq!(err.to_string(), "authentication failed: invalid credentials");
    }

    #[test]
    fn error_display_timeout() {
        let err = Error::Timeout(TimeoutError::Action {
            elapsed: Duration::from_secs(5),
        });
        assert_eq!(err.to_string(), "operation timed out: action timed out after 5s");
    }

    #[test]
    fn error_display_protocol() {
        let err = Error::Protocol(ProtocolError::MalformedMessage {
            details: "missing header".into(),
        });
        assert_eq!(err.to_string(), "protocol error: malformed message: missing header");
    }

    #[test]
    fn error_display_io() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
        let err = Error::Io(io_err);
        assert_eq!(err.to_string(), "I/O error: pipe broke");
    }

    // --- ConnectionError Display ---

    #[test]
    fn connection_error_connect_failed() {
        let source = std::io::Error::new(std::io::ErrorKind::ConnectionRefused, "refused");
        let err = ConnectionError::ConnectFailed {
            address: "127.0.0.1:5038".into(),
            source,
        };
        assert_eq!(
            err.to_string(),
            "failed to connect to 127.0.0.1:5038: refused"
        );
    }

    #[test]
    fn connection_error_closed() {
        let err = ConnectionError::Closed;
        assert_eq!(err.to_string(), "connection closed unexpectedly");
    }

    #[test]
    fn connection_error_tls() {
        let err = ConnectionError::Tls("certificate expired".into());
        assert_eq!(err.to_string(), "TLS handshake failed: certificate expired");
    }

    #[test]
    fn connection_error_dns_resolution() {
        let source = std::io::Error::new(std::io::ErrorKind::Other, "NXDOMAIN");
        let err = ConnectionError::DnsResolution {
            host: "pbx.example.com".into(),
            source,
        };
        assert_eq!(
            err.to_string(),
            "DNS resolution failed for pbx.example.com: NXDOMAIN"
        );
    }

    // --- AuthError Display ---

    #[test]
    fn auth_error_rejected() {
        let err = AuthError::Rejected {
            reason: "bad password".into(),
        };
        assert_eq!(err.to_string(), "login rejected: bad password");
    }

    #[test]
    fn auth_error_invalid_credentials() {
        let err = AuthError::InvalidCredentials;
        assert_eq!(err.to_string(), "invalid credentials");
    }

    #[test]
    fn auth_error_challenge_failed() {
        let err = AuthError::ChallengeFailed;
        assert_eq!(err.to_string(), "challenge-response failed");
    }

    // --- TimeoutError Display ---

    #[test]
    fn timeout_error_action() {
        let err = TimeoutError::Action {
            elapsed: Duration::from_millis(3500),
        };
        assert_eq!(err.to_string(), "action timed out after 3.5s");
    }

    #[test]
    fn timeout_error_connection() {
        let err = TimeoutError::Connection {
            elapsed: Duration::from_secs(30),
        };
        assert_eq!(err.to_string(), "connection timed out after 30s");
    }

    #[test]
    fn timeout_error_sub_millisecond() {
        let err = TimeoutError::Action {
            elapsed: Duration::from_micros(500),
        };
        // Duration debug uses µs for sub-ms values
        assert!(err.to_string().contains("500"));
    }

    // --- ProtocolError Display ---

    #[test]
    fn protocol_error_malformed_message() {
        let err = ProtocolError::MalformedMessage {
            details: "unexpected EOF".into(),
        };
        assert_eq!(err.to_string(), "malformed message: unexpected EOF");
    }

    #[test]
    fn protocol_error_unexpected_response() {
        let err = ProtocolError::UnexpectedResponse {
            expected: "Success".into(),
            actual: "Error".into(),
        };
        assert_eq!(
            err.to_string(),
            "unexpected response: expected Success, got Error"
        );
    }

    #[test]
    fn protocol_error_unsupported_version() {
        let err = ProtocolError::UnsupportedVersion {
            version: "99.0".into(),
        };
        assert_eq!(err.to_string(), "unsupported protocol version: 99.0");
    }

    // --- From impls ---

    #[test]
    fn from_connection_error() {
        let inner = ConnectionError::Closed;
        let err: Error = inner.into();
        assert!(matches!(err, Error::Connection(ConnectionError::Closed)));
    }

    #[test]
    fn from_auth_error() {
        let inner = AuthError::ChallengeFailed;
        let err: Error = inner.into();
        assert!(matches!(err, Error::Auth(AuthError::ChallengeFailed)));
    }

    #[test]
    fn from_timeout_error() {
        let inner = TimeoutError::Action {
            elapsed: Duration::from_secs(1),
        };
        let err: Error = inner.into();
        assert!(matches!(err, Error::Timeout(TimeoutError::Action { .. })));
    }

    #[test]
    fn from_protocol_error() {
        let inner = ProtocolError::MalformedMessage {
            details: "x".into(),
        };
        let err: Error = inner.into();
        assert!(matches!(
            err,
            Error::Protocol(ProtocolError::MalformedMessage { .. })
        ));
    }

    #[test]
    fn from_io_error() {
        let inner = std::io::Error::new(std::io::ErrorKind::NotFound, "gone");
        let err: Error = inner.into();
        assert!(matches!(err, Error::Io(_)));
    }

    // --- edge cases ---

    #[test]
    fn connection_error_empty_address() {
        let source = std::io::Error::new(std::io::ErrorKind::InvalidInput, "bad");
        let err = ConnectionError::ConnectFailed {
            address: String::new(),
            source,
        };
        assert_eq!(err.to_string(), "failed to connect to : bad");
    }

    #[test]
    fn auth_error_rejected_empty_reason() {
        let err = AuthError::Rejected {
            reason: String::new(),
        };
        assert_eq!(err.to_string(), "login rejected: ");
    }

    #[test]
    fn protocol_error_empty_details() {
        let err = ProtocolError::MalformedMessage {
            details: String::new(),
        };
        assert_eq!(err.to_string(), "malformed message: ");
    }
}