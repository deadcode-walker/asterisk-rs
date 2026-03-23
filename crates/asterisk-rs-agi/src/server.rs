use std::sync::Arc;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::sync::{watch, Semaphore};

use crate::channel::AgiChannel;
use crate::error::{AgiError, Result};
use crate::handler::AgiHandler;
use crate::request::AgiRequest;

/// handle for signaling an [`AgiServer`] to shut down
#[derive(Clone)]
pub struct ShutdownHandle {
    tx: watch::Sender<bool>,
}

impl ShutdownHandle {
    /// signal the server to stop accepting connections
    pub fn shutdown(&self) {
        let _ = self.tx.send(true);
    }
}

/// FastAGI TCP server that dispatches connections to a handler
pub struct AgiServer<H: AgiHandler> {
    listener: TcpListener,
    handler: Arc<H>,
    max_connections: Option<usize>,
    shutdown_rx: watch::Receiver<bool>,
}

/// builder for configuring and constructing an [`AgiServer`]
#[must_use]
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
    /// runs until shutdown is signaled or an unrecoverable error occurs
    pub async fn run(mut self) -> Result<()> {
        let semaphore = self.max_connections.map(|n| Arc::new(Semaphore::new(n)));

        loop {
            tokio::select! {
                result = self.listener.accept() => {
                    let (stream, peer) = match result {
                        Ok(conn) => conn,
                        Err(err) => {
                            tracing::warn!(%err, "failed to accept connection");
                            // brief backoff prevents CPU spin on persistent errors (EMFILE/ENFILE)
                            tokio::time::sleep(Duration::from_millis(100)).await;
                            continue;
                        }
                    };

                    tracing::debug!(%peer, "new AGI connection");

                    let handler = Arc::clone(&self.handler);

                    // acquire a permit if max_connections is configured; race against shutdown
                    // so a saturated server still responds promptly to stop signals
                    let permit = if let Some(sem) = &semaphore {
                        let acquire = sem.clone().acquire_owned();
                        tokio::select! {
                            result = acquire => match result {
                                Ok(p) => Some(p),
                                Err(_) => {
                                    // semaphore closed — should not happen during normal operation
                                    tracing::error!("connection semaphore closed unexpectedly");
                                    return Err(AgiError::Io(std::io::Error::other(
                                        "connection semaphore closed",
                                    )));
                                }
                            },
                            _ = self.shutdown_rx.changed() => {
                                tracing::info!("AGI server shutting down");
                                return Ok(());
                            }
                        }
                    } else {
                        None
                    };

                    tokio::spawn(async move {
                        // permit is held until the task completes, then dropped automatically
                        let _permit = permit;

                        if let Err(err) = handle_connection(handler, stream).await {
                            tracing::warn!(%peer, %err, "AGI session error");
                        }
                    });
                }
                result = self.shutdown_rx.changed() => {
                    // Err means all senders were dropped — treat as shutdown signal
                    if result.is_err() || *self.shutdown_rx.borrow() {
                        tracing::info!("AGI server shutting down");
                        return Ok(());
                    }
                }
            }
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

    // 30s deadline prevents slow/malicious clients from holding a connection slot indefinitely
    let request = match tokio::time::timeout(
        Duration::from_secs(30),
        AgiRequest::parse_from_reader(&mut reader),
    )
    .await
    {
        Ok(result) => result?,
        Err(_elapsed) => {
            tracing::warn!("AGI prelude read timed out after 30s");
            return Ok(());
        }
    };

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
    ///
    /// returns the server and a handle that can signal graceful shutdown
    pub async fn build(self) -> Result<(AgiServer<H>, ShutdownHandle)> {
        let handler = self.handler.ok_or_else(|| AgiError::InvalidConfig {
            details: "handler is required".to_owned(),
        })?;

        let listener = TcpListener::bind(&self.bind_addr).await?;
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        tracing::info!(addr = %self.bind_addr, "FastAGI server bound");

        let server = AgiServer {
            listener,
            handler: Arc::new(handler),
            max_connections: self.max_connections,
            shutdown_rx,
        };

        let handle = ShutdownHandle { tx: shutdown_tx };

        Ok((server, handle))
    }
}
