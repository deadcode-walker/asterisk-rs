use std::sync::Arc;
use std::time::Duration;

use tokio::io::BufReader;
use tokio::net::TcpListener;
use tokio::sync::{Semaphore, watch};
use tokio::task::JoinSet;

use crate::channel::AgiChannel;
use crate::error::{AgiError, Result};
use crate::handler::AgiHandler;
use crate::request::AgiRequest;

const DEFAULT_MAX_CONNECTIONS: usize = 256;
const DEFAULT_SHUTDOWN_TIMEOUT: Duration = Duration::from_secs(5);

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
    max_connections: usize,
    shutdown_timeout: Duration,
    shutdown_rx: watch::Receiver<bool>,
}

/// builder for configuring and constructing an [`AgiServer`]
#[must_use]
pub struct AgiServerBuilder<H> {
    bind_addr: String,
    handler: Option<H>,
    max_connections: usize,
    shutdown_timeout: Duration,
    allow_external_bind: bool,
}

impl<H: AgiHandler> AgiServer<H> {
    /// create a new builder for configuring the server
    pub fn builder() -> AgiServerBuilder<H> {
        AgiServerBuilder {
            bind_addr: "127.0.0.1:4573".to_owned(),
            handler: None,
            max_connections: DEFAULT_MAX_CONNECTIONS,
            shutdown_timeout: DEFAULT_SHUTDOWN_TIMEOUT,
            allow_external_bind: false,
        }
    }

    /// return the bound listener address
    pub fn local_addr(&self) -> Result<std::net::SocketAddr> {
        Ok(self.listener.local_addr()?)
    }

    /// accept connections and dispatch them to the handler
    ///
    /// runs until shutdown is signaled or an unrecoverable error occurs
    pub async fn run(mut self) -> Result<()> {
        let semaphore = Arc::new(Semaphore::new(self.max_connections));
        let mut sessions = JoinSet::new();

        loop {
            let permit = tokio::select! {
                result = semaphore.clone().acquire_owned() => result.map_err(|_| {
                    AgiError::Io(std::io::Error::other("connection semaphore closed"))
                })?,
                result = self.shutdown_rx.changed() => {
                    if result.is_err() || *self.shutdown_rx.borrow() {
                        break;
                    }
                    continue;
                }
            };

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
                    sessions.spawn(async move {
                        let _permit = permit;

                        if let Err(err) = handle_connection(handler, stream, peer).await {
                            tracing::warn!(%peer, %err, "AGI session error");
                        }
                    });
                }
                result = self.shutdown_rx.changed() => {
                    if result.is_err() || *self.shutdown_rx.borrow() {
                        break;
                    }
                }
                Some(result) = sessions.join_next(), if !sessions.is_empty() => {
                    if let Err(error) = result {
                        tracing::error!(%error, "AGI session task failed");
                    }
                }
            }
        }

        tracing::info!("AGI server shutting down");
        let drain = async {
            while let Some(result) = sessions.join_next().await {
                if let Err(error) = result {
                    tracing::error!(%error, "AGI session task failed during shutdown");
                }
            }
        };
        if tokio::time::timeout(self.shutdown_timeout, drain)
            .await
            .is_err()
        {
            tracing::warn!(?self.shutdown_timeout, "AGI shutdown drain timed out");
            sessions.abort_all();
            while sessions.join_next().await.is_some() {}
        }
        Ok(())
    }
}

/// process a single AGI connection: read environment, create channel, dispatch to handler
async fn handle_connection<H: AgiHandler>(
    handler: Arc<H>,
    stream: tokio::net::TcpStream,
    peer: std::net::SocketAddr,
) -> Result<()> {
    let (read_half, write_half) = stream.into_split();
    let mut reader = BufReader::new(read_half);

    // 30s deadline prevents slow/malicious clients from holding a connection slot indefinitely
    let mut request = match tokio::time::timeout(
        Duration::from_secs(30),
        AgiRequest::parse_from_reader(&mut reader),
    )
    .await
    {
        Ok(result) => result?,
        Err(_elapsed) => {
            tracing::warn!("AGI prelude read timed out after 30s");
            return Err(AgiError::RequestTimeout {
                elapsed: Duration::from_secs(30),
            });
        }
    };
    request.set_peer_addr(peer);

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
        self.max_connections = n;
        self
    }

    /// set how long shutdown waits for active handlers before cancelling them
    pub fn shutdown_timeout(mut self, timeout: Duration) -> Self {
        self.shutdown_timeout = timeout;
        self
    }

    /// explicitly allow binding beyond the local host
    ///
    /// external FastAGI must be protected by a private network, firewall, or
    /// authenticated TLS proxy because the protocol has no native authentication
    pub fn allow_external_bind(mut self, allow: bool) -> Self {
        self.allow_external_bind = allow;
        self
    }

    /// build the server, binding the TCP listener
    ///
    /// returns the server and a handle that can signal graceful shutdown
    pub async fn build(self) -> Result<(AgiServer<H>, ShutdownHandle)> {
        let handler = self.handler.ok_or_else(|| AgiError::InvalidConfig {
            details: "handler is required".to_owned(),
        })?;
        if self.max_connections == 0 {
            return Err(AgiError::InvalidConfig {
                details: "max_connections must be greater than zero".to_owned(),
            });
        }
        if self.shutdown_timeout.is_zero() {
            return Err(AgiError::InvalidConfig {
                details: "shutdown_timeout must be greater than zero".to_owned(),
            });
        }

        let listener = TcpListener::bind(&self.bind_addr).await?;
        let local_addr = listener.local_addr()?;
        if !local_addr.ip().is_loopback() && !self.allow_external_bind {
            return Err(AgiError::InvalidConfig {
                details:
                    "external bind requires allow_external_bind(true) and network authentication"
                        .to_owned(),
            });
        }
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        tracing::info!(addr = %self.bind_addr, "FastAGI server bound");

        let server = AgiServer {
            listener,
            handler: Arc::new(handler),
            max_connections: self.max_connections,
            shutdown_timeout: self.shutdown_timeout,
            shutdown_rx,
        };

        let handle = ShutdownHandle { tx: shutdown_tx };

        Ok((server, handle))
    }
}
