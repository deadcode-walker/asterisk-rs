//! AMI-specific error types.

use asterisk_rs_core::error::{AuthError, ConnectionError, ProtocolError, TimeoutError};

/// errors specific to AMI operations
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
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
    ActionFailed { action_id: String, message: String },

    #[error("client is disconnected")]
    Disconnected,

    #[error("action response channel closed")]
    ResponseChannelClosed,

    /// the action reached the wire, but no definitive response was received
    #[error("outcome unknown for AMI action {action_id}: it may have been executed")]
    OutcomeUnknown { action_id: String },

    #[error("AMI in-flight action limit exceeded ({limit})")]
    InFlightLimitExceeded { limit: usize },

    #[error("AMI event list {action_id} exceeded the {limit}-event limit")]
    EventListEventLimitExceeded { action_id: String, limit: usize },

    #[error("AMI event list {action_id} exceeded the {limit}-byte limit")]
    EventListByteLimitExceeded { action_id: String, limit: usize },

    #[error("AMI event-list in-flight action limit exceeded ({limit})")]
    EventListInFlightLimitExceeded { limit: usize },

    #[error(
        "AMI event lists exceeded the connection-wide {limit}-byte limit while collecting {action_id}"
    )]
    EventListConnectionByteLimitExceeded { action_id: String, limit: usize },

    #[error("AMI event list {action_id} was cancelled by Asterisk")]
    EventListCancelled { action_id: String },

    #[error("invalid configuration: {details}")]
    InvalidConfig { details: String },
}

pub type Result<T> = std::result::Result<T, AmiError>;
