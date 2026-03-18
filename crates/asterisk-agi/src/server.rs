use std::sync::Arc;

use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::sync::Semaphore;

use crate::channel::AgiChannel;
use crate::error::{AgiError, Result};
use crate::handler::AgiHandler;
use crate::request::AgiRequest;

/// FastAGI TCP server that dispatches connections to a handler
pub struct AgiServer<H: AgiHandler> {
    listener: TcpListener,
    handler: Arc<H>,
    max_connections: Option<usize>,
}

/// builder for configuring and constructing an [`AgiServer`]
pub struct AgiServerBuilder<H> {
    bind_addr: String,
    handler: Option<H>,
    max_connections: Option<usize>,
}

impl<H: AgiHandler> AgiServer<H> {
    /// create a new builder for configuring the server
    pub fn builder() -> AgiServerBuilder<H> {
        AgiServerBuilder {
            bind_addr: "0.0.0.0:4573".to_owned(),
            handler: None,
            max_connections: None,
        }
    }

    /// accept connections and dispatch them to the handler
    ///
    /// runs indefinitely until an unrecoverable error occurs
    pub async fn run(self) -> Result<()> {
        let semaphore = self.max_connections.map(|n| Arc::new(Semaphore::new(n)));

        loop {
            let (stream, peer) = match self.listener.accept().await {
                Ok(conn) => conn,
                Err(err) => {
                    tracing::warn!(%err, "failed to accept connection");
                    continue;
                }
            };

            tracing::debug!(%peer, "new AGI connection");

            let handler = Arc::clone(&self.handler);
            let permit = match &semaphore {
                Some(sem) => match sem.clone().acquire_owned().await {
                    Ok(p) => Some(p),
                    Err(_) => {
                        // semaphore closed — should not happen during normal operation
                        tracing::error!("connection semaphore closed unexpectedly");
                        return Err(AgiError::Io(std::io::Error::other(
                            "connection semaphore closed",
                        )));
                    }
                },
                None => None,
            };

            tokio::spawn(async move {
                // permit is held until the task completes, then dropped automatically
                let _permit = permit;

                if let Err(err) = handle_connection(handler, stream).await {
                    tracing::warn!(%peer, %err, "AGI session error");
                }
            });
        }
    }
}

/// process a single AGI connection: read environment, create channel, dispatch to handler
async fn handle_connection<H: AgiHandler>(
    handler: Arc<H>,
    stream: tokio::net::TcpStream,
) -> Result<()> {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // read the AGI environment variables sent by asterisk
    let request = AgiRequest::parse_from_reader(&mut reader).await?;

    let channel = AgiChannel::new(reader, write_half);
    handler.handle(request, channel).await
}

impl<H: AgiHandler> AgiServerBuilder<H> {
    /// set the address to bind the TCP listener to
    pub fn bind(mut self, addr: impl Into<String>) -> Self {
        self.bind_addr = addr.into();
        self
    }

    /// set the handler for incoming AGI sessions
    pub fn handler(mut self, handler: H) -> Self {
        self.handler = Some(handler);
        self
    }

    /// set the maximum number of concurrent connections
    pub fn max_connections(mut self, n: usize) -> Self {
        self.max_connections = Some(n);
        self
    }

    /// build the server, binding the TCP listener
    pub async fn build(self) -> Result<AgiServer<H>> {
        let handler = self
            .handler
            .expect("handler is required — call .handler() before .build()");

        let listener = TcpListener::bind(&self.bind_addr).await?;

        tracing::info!(addr = %self.bind_addr, "FastAGI server bound");

        Ok(AgiServer {
            listener,
            handler: Arc::new(handler),
            max_connections: self.max_connections,
        })
    }
}
