use crate::channel::AgiChannel;
use crate::request::AgiRequest;

/// trait for handling incoming AGI connections
pub trait AgiHandler: Send + Sync + 'static {
    /// handle an AGI session
    fn handle(
        &self,
        request: AgiRequest,
        channel: AgiChannel,
    ) -> impl std::future::Future<Output = crate::error::Result<()>> + Send;
}
