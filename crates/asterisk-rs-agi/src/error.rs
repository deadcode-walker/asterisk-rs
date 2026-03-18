use asterisk_rs_core::error::ProtocolError;

/// errors specific to the AGI protocol
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum AgiError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("channel hung up during AGI session")]
    ChannelHungUp,

    #[error("invalid AGI response: {raw}")]
    InvalidResponse { raw: String },

    #[error("AGI command failed with code {code}: {message}")]
    CommandFailed { code: u16, message: String },

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

pub type Result<T> = std::result::Result<T, AgiError>;


#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
        let err = AgiError::Io(io_err);
        let msg = err.to_string();
        assert!(msg.contains("pipe broke"), "expected io error details in: {msg}");
    }

    #[test]
    fn channel_hung_up_display() {
        let err = AgiError::ChannelHungUp;
        assert_eq!(err.to_string(), "channel hung up during AGI session");
    }

    #[test]
    fn invalid_response_display() {
        let err = AgiError::InvalidResponse {
            raw: "garbage".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("garbage"), "expected raw string in: {msg}");
    }

    #[test]
    fn command_failed_display() {
        let err = AgiError::CommandFailed {
            code: 510,
            message: "invalid command".to_owned(),
        };
        let msg = err.to_string();
        assert!(msg.contains("510"), "expected code in: {msg}");
        assert!(msg.contains("invalid command"), "expected message in: {msg}");
    }

    #[test]
    fn protocol_error_display() {
        let proto = ProtocolError::MalformedMessage {
            details: "bad frame".to_owned(),
        };
        let err = AgiError::Protocol(proto);
        let msg = err.to_string();
        assert!(msg.contains("bad frame"), "expected protocol details in: {msg}");
    }

    #[test]
    fn errors_are_send_sync() {
        fn assert_send_sync<T: Send + Sync>() {}
        assert_send_sync::<AgiError>();
    }

    #[test]
    fn io_error_from_conversion() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
        let err: AgiError = io_err.into();
        assert!(matches!(err, AgiError::Io(_)));
    }

    #[test]
    fn protocol_error_from_conversion() {
        let proto = ProtocolError::UnsupportedVersion {
            version: "99".to_owned(),
        };
        let err: AgiError = proto.into();
        assert!(matches!(err, AgiError::Protocol(_)));
    }
}