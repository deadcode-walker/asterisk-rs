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

    #[error("invalid AGI argument: {details}")]
    InvalidArgument { details: String },

    #[error("command already in flight; channel is not cancel-safe")]
    CommandInFlight,

    #[error("channel is poisoned due to a previous I/O error")]
    ChannelPoisoned,

    #[error("protocol error: {0}")]
    Protocol(#[from] ProtocolError),
}

pub type Result<T> = std::result::Result<T, AgiError>;
