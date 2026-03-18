//! AMI-specific error types.

use asterisk_rs_core::error::{AuthError, ConnectionError, ProtocolError, TimeoutError};

/// errors specific to AMI operations
#[derive(Debug, thiserror::Error)]
pub enum AmiError {
    #[error("connection error: {0}")]
    Connection(#[from] ConnectionError),

    #[error("authentication error: {0}")]
    Auth(#[from] AuthError),

    #[error("timeout: {0}")]
    Timeout(#[from] TimeoutError),

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),

    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),

    #[error("action failed: {message}")]
    ActionFailed {
        action_id: String,
        message: String,
    },

    #[error("client is disconnected")]
    Disconnected,

    #[error("action response channel closed")]
    ResponseChannelClosed,
}

pub type Result<T> = std::result::Result<T, AmiError>;
