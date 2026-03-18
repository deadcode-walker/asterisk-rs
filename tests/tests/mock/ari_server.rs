//! mock ARI server for integration testing
//!
//! serves both HTTP REST and WebSocket on a single port using raw TCP.
//! HTTP requests are matched against a pre-registered route table.
//! WebSocket clients receive events pushed via broadcast channel.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{broadcast, watch};
use tokio_tungstenite::tungstenite::Message;

/// pre-configured response for a given (method, path) pair
#[derive(Clone, Debug)]
pub struct MockRoute {
    pub status: u16,
    pub body: String,
}

/// shared state visible to all connection handlers
struct ServerState {
    routes: HashMap<(String, String), MockRoute>,
    event_tx: broadcast::Sender<String>,
}

/// mock ARI server binding HTTP and WebSocket on one port
pub struct MockAriServer {
    addr: SocketAddr,
    event_tx: broadcast::Sender<String>,
    shutdown_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
}

impl MockAriServer {
    pub fn builder() -> MockAriServerBuilder {
        MockAriServerBuilder::new()
    }

    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// push a JSON event string to all connected websocket clients
    pub fn send_event(&self, json: &str) {
        // ignore error when no receivers are connected
        let _ = self.event_tx.send(json.to_string());
    }

    /// shut down the accept loop and abort all handlers
    pub fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        self.task.abort();
    }
}

/// builder for [`MockAriServer`] with route registration
pub struct MockAriServerBuilder {
    routes: HashMap<(String, String), MockRoute>,
}

impl MockAriServerBuilder {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
        }
    }

    /// register a canned response for (method, path)
    pub fn route(mut self, method: &str, path: &str, status: u16, body: &str) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_string()),
            MockRoute {
                status,
                body: body.to_string(),
            },
        );
        self
    }

    /// bind to a random port and start accepting connections
    pub async fn start(self) -> MockAriServer {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("failed to bind mock ARI listener");
        let addr = listener
            .local_addr()
            .expect("failed to get mock ARI local addr");

        let (event_tx, _) = broadcast::channel::<String>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let state = Arc::new(ServerState {
            routes: self.routes,
            event_tx: event_tx.clone(),
        });

        let task = tokio::spawn(accept_loop(listener, state, shutdown_rx));

        MockAriServer {
            addr,
            event_tx,
            shutdown_tx,
            task,
        }
    }
}

/// accept incoming TCP connections until shutdown is signaled
async fn accept_loop(
    listener: TcpListener,
    state: Arc<ServerState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    loop {
        tokio::select! {
            result = listener.accept() => {
                match result {
                    Ok((stream, _peer)) => {
                        let st = Arc::clone(&state);
                        tokio::spawn(handle_connection(stream, st));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "mock ARI accept error");
                    }
                }
            }
            _ = shutdown_rx.changed() => {
                if *shutdown_rx.borrow() {
                    return;
                }
            }
        }
    }
}

/// route a single TCP connection to either websocket or HTTP handling
async fn handle_connection(stream: TcpStream, state: Arc<ServerState>) {
    // peek at the request to decide protocol without consuming bytes
    let mut buf = [0u8; 4096];
    let n = match stream.peek(&mut buf).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "mock ARI peek failed");
            return;
        }
    };

    let request_preview = String::from_utf8_lossy(&buf[..n]);
    let lower = request_preview.to_lowercase();

    if lower.contains("upgrade: websocket") {
        handle_websocket(stream, state).await;
    } else {
        handle_http(stream, state).await;
    }
}

/// perform websocket handshake and stream events until client disconnects
async fn handle_websocket(stream: TcpStream, state: Arc<ServerState>) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!(error = %e, "mock ARI ws handshake failed");
            return;
        }
    };

    let (mut write, mut read) = ws.split();
    let mut event_rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            event = event_rx.recv() => {
                match event {
                    Ok(json) => {
                        if write.send(Message::Text(json)).await.is_err() {
                            break;
                        }
                    }
                    // sender dropped or lagged — stop
                    Err(_) => break,
                }
            }
            msg = read.next() => {
                match msg {
                    Some(Ok(Message::Close(_))) | None => break,
                    Some(Err(_)) => break,
                    _ => {} // ignore pings, text from client, etc.
                }
            }
        }
    }
}

/// parse an HTTP request from the stream and send a canned response
async fn handle_http(mut stream: TcpStream, state: Arc<ServerState>) {
    let mut buf = vec![0u8; 8192];
    let n = match stream.read(&mut buf).await {
        Ok(n) => n,
        Err(e) => {
            tracing::warn!(error = %e, "mock ARI http read failed");
            return;
        }
    };

    let request = String::from_utf8_lossy(&buf[..n]);

    // parse request line: "METHOD /path HTTP/1.1"
    let first_line = request.lines().next().unwrap_or_default();
    let parts: Vec<&str> = first_line.splitn(3, ' ').collect();
    let method = parts.first().copied().unwrap_or_default().to_uppercase();
    let path = parts.get(1).copied().unwrap_or_default().to_string();

    let key = (method, path);
    let route = state.routes.get(&key).cloned().unwrap_or(MockRoute {
        status: 404,
        body: r#"{"message":"not found"}"#.to_string(),
    });

    let reason = status_reason(route.status);
    let headers = format!(
        "Content-Type: application/json\r\nContent-Length: {}\r\nConnection: close",
        route.body.len(),
    );
    let response =
        format!("HTTP/1.1 {status} {reason}\r\n{headers}\r\n\r\n{body}",
            status = route.status, body = route.body,
        );

    let _ = stream.write_all(response.as_bytes()).await;
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}
