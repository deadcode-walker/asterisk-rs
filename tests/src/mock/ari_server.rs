//! mock ARI server for integration testing
//!
//! serves both HTTP REST and WebSocket on a single port using raw TCP.
//! HTTP requests are matched against a pre-registered route table.
//! WebSocket clients receive events pushed via broadcast channel.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::atomic::{AtomicUsize, Ordering};

use futures_util::{SinkExt, StreamExt};
use tokio::io::{AsyncReadExt, AsyncWriteExt};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::{Notify, broadcast, watch};
use tokio_tungstenite::tungstenite::Message;

/// pre-configured response for a given (method, path) pair
#[derive(Clone, Debug)]
pub struct MockRoute {
    pub status: u16,
    pub body: String,
    framing: ResponseFraming,
    before_response: Vec<Message>,
    response_delay: std::time::Duration,
}

#[derive(Clone, Copy, Debug)]
enum ResponseFraming {
    Fixed,
    Chunked,
    Disconnect,
}

/// parsed HTTP request captured at the mock transport boundary
#[derive(Clone)]
pub struct MockRequest {
    pub method: String,
    pub path: String,
    pub content_length: usize,
    pub authorization: Option<String>,
    pub content_type: Option<String>,
    pub body: Vec<u8>,
}

impl std::fmt::Debug for MockRequest {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("MockRequest")
            .field("method", &self.method)
            .field("path", &self.path)
            .field("content_length", &self.content_length)
            .field(
                "authorization",
                &self.authorization.as_ref().map(|_| "[redacted]"),
            )
            .field("content_type", &self.content_type)
            .field("body", &String::from_utf8_lossy(&self.body))
            .finish()
    }
}

/// shared state visible to all connection handlers
struct ServerState {
    routes: HashMap<(String, String), MockRoute>,
    event_tx: broadcast::Sender<Message>,
    ws_clients: AtomicUsize,
    ws_connections: AtomicUsize,
    ws_connected: Notify,
    ws_requests: Mutex<Vec<serde_json::Value>>,
    ws_request_received: Notify,
    requests: Mutex<Vec<MockRequest>>,
    request_received: Notify,
}

/// mock ARI server binding HTTP and WebSocket on one port
pub struct MockAriServer {
    addr: SocketAddr,
    event_tx: broadcast::Sender<Message>,
    shutdown_tx: watch::Sender<bool>,
    task: tokio::task::JoinHandle<()>,
    state: Arc<ServerState>,
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
        self.send_ws_message(Message::Text(json.to_owned().into()));
    }

    /// push an exact WebSocket frame to all connected clients
    pub fn send_ws_message(&self, message: Message) {
        // An absent receiver is a useful no-op for tests that arrange frames
        // before connecting.
        let _ = self.event_tx.send(message);
    }

    /// number of WebSocket connections currently handled by the server
    pub fn active_ws_connections(&self) -> usize {
        self.state.ws_clients.load(Ordering::Acquire)
    }

    /// total WebSocket connections accepted over the server lifetime
    pub fn total_ws_connections(&self) -> usize {
        self.state.ws_connections.load(Ordering::Acquire)
    }

    /// snapshot decoded unified REST request envelopes
    pub fn ws_requests(&self) -> Vec<serde_json::Value> {
        self.state
            .ws_requests
            .lock()
            .expect("mock WebSocket request mutex poisoned")
            .clone()
    }

    /// wait until at least `count` unified REST request envelopes are captured
    pub async fn wait_for_ws_requests(&self, count: usize) {
        loop {
            let notified = self.state.ws_request_received.notified();
            if self
                .state
                .ws_requests
                .lock()
                .expect("mock WebSocket request mutex poisoned")
                .len()
                >= count
            {
                return;
            }
            notified.await;
        }
    }

    /// signal active handlers to close, then stop the accept loop
    pub async fn shutdown(self) {
        let _ = self.shutdown_tx.send(true);
        let mut task = self.task;
        match tokio::time::timeout(std::time::Duration::from_secs(2), &mut task).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => panic!("mock ARI accept task failed: {error}"),
            Err(_) => {
                task.abort();
                let _ = task.await;
                panic!("mock ARI server did not join within two seconds");
            }
        }
    }

    /// wait until at least one websocket client has connected
    pub async fn wait_for_ws_client(&self) {
        loop {
            // register the notification future before the load to avoid a
            // race where the signal fires between the check and the await
            let notified = self.state.ws_connected.notified();
            if self.state.ws_clients.load(Ordering::Acquire) > 0 {
                return;
            }
            notified.await;
        }
    }

    /// snapshot parsed HTTP requests received by the mock server
    pub fn requests(&self) -> Vec<MockRequest> {
        self.state
            .requests
            .lock()
            .expect("mock request mutex poisoned")
            .clone()
    }

    /// wait until at least `count` HTTP requests have been captured
    pub async fn wait_for_requests(&self, count: usize) {
        loop {
            let notified = self.state.request_received.notified();
            if self
                .state
                .requests
                .lock()
                .expect("mock request mutex poisoned")
                .len()
                >= count
            {
                return;
            }
            notified.await;
        }
    }
}

/// builder for [`MockAriServer`] with route registration
pub struct MockAriServerBuilder {
    routes: HashMap<(String, String), MockRoute>,
    bind_addr: SocketAddr,
}

impl Default for MockAriServerBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl MockAriServerBuilder {
    pub fn new() -> Self {
        Self {
            routes: HashMap::new(),
            bind_addr: "127.0.0.1:0".parse().expect("valid mock bind address"),
        }
    }

    /// register a canned response for (method, path)
    pub fn route(mut self, method: &str, path: &str, status: u16, body: &str) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_string()),
            MockRoute {
                status,
                body: body.to_string(),
                framing: ResponseFraming::Fixed,
                before_response: Vec::new(),
                response_delay: std::time::Duration::ZERO,
            },
        );
        self
    }

    /// register a response encoded with HTTP chunked transfer framing
    pub fn route_chunked(mut self, method: &str, path: &str, status: u16, body: &str) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_string()),
            MockRoute {
                status,
                body: body.to_owned(),
                framing: ResponseFraming::Chunked,
                before_response: Vec::new(),
                response_delay: std::time::Duration::ZERO,
            },
        );
        self
    }

    /// capture a request and close the connection before response headers
    pub fn route_disconnect(mut self, method: &str, path: &str) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_owned()),
            MockRoute {
                status: 500,
                body: String::new(),
                framing: ResponseFraming::Disconnect,
                before_response: Vec::new(),
                response_delay: std::time::Duration::ZERO,
            },
        );
        self
    }

    /// register a response that emits exact WebSocket frames before replying
    pub fn route_after_ws_messages(
        mut self,
        method: &str,
        path: &str,
        status: u16,
        body: &str,
        messages: Vec<Message>,
    ) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_owned()),
            MockRoute {
                status,
                body: body.to_owned(),
                framing: ResponseFraming::Fixed,
                before_response: messages,
                response_delay: std::time::Duration::ZERO,
            },
        );
        self
    }

    /// register a response held long enough for a test to inject pre-response events
    pub fn route_delayed(
        mut self,
        method: &str,
        path: &str,
        status: u16,
        body: &str,
        delay: std::time::Duration,
    ) -> Self {
        self.routes.insert(
            (method.to_uppercase(), path.to_owned()),
            MockRoute {
                status,
                body: body.to_owned(),
                framing: ResponseFraming::Fixed,
                before_response: Vec::new(),
                response_delay: delay,
            },
        );
        self
    }

    /// bind the mock server to an exact address
    pub fn bind(mut self, addr: SocketAddr) -> Self {
        self.bind_addr = addr;
        self
    }

    /// bind to a random port and start accepting connections
    pub async fn start(self) -> MockAriServer {
        let listener = TcpListener::bind(self.bind_addr)
            .await
            .expect("failed to bind mock ARI listener");
        let addr = listener
            .local_addr()
            .expect("failed to get mock ARI local addr");

        let (event_tx, _) = broadcast::channel::<Message>(64);
        let (shutdown_tx, shutdown_rx) = watch::channel(false);

        let state = Arc::new(ServerState {
            routes: self.routes,
            event_tx: event_tx.clone(),
            ws_clients: AtomicUsize::new(0),
            ws_connections: AtomicUsize::new(0),
            ws_connected: Notify::new(),
            ws_requests: Mutex::new(Vec::new()),
            ws_request_received: Notify::new(),
            requests: Mutex::new(Vec::new()),
            request_received: Notify::new(),
        });

        let task = tokio::spawn(accept_loop(listener, Arc::clone(&state), shutdown_rx));

        MockAriServer {
            addr,
            event_tx,
            shutdown_tx,
            task,
            state,
        }
    }
}

/// accept incoming TCP connections until shutdown is signaled
async fn accept_loop(
    listener: TcpListener,
    state: Arc<ServerState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let mut handlers = tokio::task::JoinSet::new();
    loop {
        tokio::select! {
            biased;
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    break;
                }
            }
            result = handlers.join_next(), if !handlers.is_empty() => {
                if let Some(Err(error)) = result {
                    panic!("mock ARI connection task failed: {error}");
                }
            }
            result = listener.accept() => {
                match result {
                    Ok((stream, _peer)) => {
                        let st = Arc::clone(&state);
                        handlers.spawn(handle_connection(stream, st, shutdown_rx.clone()));
                    }
                    Err(e) => {
                        tracing::warn!(error = %e, "mock ARI accept error");
                    }
                }
            }
        }
    }

    drop(listener);
    let drain = async {
        while let Some(result) = handlers.join_next().await {
            if let Err(error) = result {
                panic!("mock ARI connection task failed: {error}");
            }
        }
    };
    if tokio::time::timeout(std::time::Duration::from_secs(1), drain)
        .await
        .is_err()
    {
        handlers.abort_all();
        while handlers.join_next().await.is_some() {}
    }
}

/// route a single TCP connection to either websocket or HTTP handling
async fn handle_connection(
    stream: TcpStream,
    state: Arc<ServerState>,
    shutdown_rx: watch::Receiver<bool>,
) {
    // Wait for the complete header block before classifying the protocol. A
    // single peek can observe a fragmented Upgrade header and route a real
    // WebSocket handshake into the HTTP handler.
    let mut buf = [0u8; 16 * 1024];
    let n = match tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let n = stream.peek(&mut buf).await?;
            if n == 0 || buf[..n].windows(4).any(|bytes| bytes == b"\r\n\r\n") {
                return std::io::Result::Ok(n);
            }
            if n == buf.len() {
                return std::io::Result::Ok(n);
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    {
        Ok(Ok(n)) => n,
        Ok(Err(error)) => {
            tracing::warn!(%error, "mock ARI peek failed");
            return;
        }
        Err(_) => {
            tracing::warn!("mock ARI request headers timed out");
            return;
        }
    };

    let is_websocket = String::from_utf8_lossy(&buf[..n])
        .split("\r\n")
        .filter_map(|line| line.split_once(':'))
        .any(|(name, value)| {
            name.eq_ignore_ascii_case("upgrade") && value.trim().eq_ignore_ascii_case("websocket")
        });

    if is_websocket {
        handle_websocket(stream, state, shutdown_rx).await;
    } else {
        handle_http(stream, state).await;
    }
}

/// perform websocket handshake and stream events until client disconnects
async fn handle_websocket(
    stream: TcpStream,
    state: Arc<ServerState>,
    mut shutdown_rx: watch::Receiver<bool>,
) {
    let ws = match tokio_tungstenite::accept_async(stream).await {
        Ok(ws) => ws,
        Err(e) => {
            tracing::warn!(error = %e, "mock ARI ws handshake failed");
            return;
        }
    };

    struct ActiveConnection<'a>(&'a AtomicUsize);
    impl Drop for ActiveConnection<'_> {
        fn drop(&mut self) {
            self.0.fetch_sub(1, Ordering::AcqRel);
        }
    }

    // signal that a ws client has connected
    state.ws_clients.fetch_add(1, Ordering::Release);
    state.ws_connections.fetch_add(1, Ordering::Release);
    let _active_connection = ActiveConnection(&state.ws_clients);
    state.ws_connected.notify_waiters();

    let (mut write, mut read) = ws.split();
    let mut event_rx = state.event_tx.subscribe();

    loop {
        tokio::select! {
            changed = shutdown_rx.changed() => {
                if changed.is_err() || *shutdown_rx.borrow() {
                    let _ = write.send(Message::Close(None)).await;
                    break;
                }
            }
            event = event_rx.recv() => {
                match event {
                    Ok(message) => {
                        if write.send(message).await.is_err() {
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
                    Some(Ok(Message::Text(text))) => {
                        let request: serde_json::Value = match serde_json::from_str(&text) {
                            Ok(request) => request,
                            Err(_) => continue,
                        };
                        if request["type"] != "RESTRequest" {
                            continue;
                        }
                        state
                            .ws_requests
                            .lock()
                            .expect("mock WebSocket request mutex poisoned")
                            .push(request.clone());
                        state.ws_request_received.notify_waiters();

                        let method = request["method"].as_str().unwrap_or_default().to_owned();
                        let uri = request["uri"].as_str().unwrap_or_default().to_owned();
                        let request_id = request["request_id"]
                            .as_str()
                            .unwrap_or_default()
                            .to_owned();
                        let path = format!("/ari/{}", uri.trim_start_matches('/'));
                        let route = state
                            .routes
                            .get(&(method, path))
                            .cloned()
                            .unwrap_or(MockRoute {
                                status: 404,
                                body: r#"{"message":"not found"}"#.to_owned(),
                                framing: ResponseFraming::Fixed,
                                before_response: Vec::new(),
                                response_delay: std::time::Duration::ZERO,
                            });
                        let response = serde_json::json!({
                            "type": "RESTResponse",
                            "status_code": route.status,
                            "reason_phrase": status_reason(route.status),
                            "uri": uri,
                            "request_id": request_id,
                            "transaction_id": "mock-transaction",
                            "content_type": "application/json",
                            "message_body": if route.body.is_empty() {
                                serde_json::Value::Null
                            } else {
                                serde_json::Value::String(route.body)
                            },
                        });
                        if write.send(Message::Text(response.to_string().into())).await.is_err() {
                            break;
                        }
                    }
                    _ => {} // ignore pings and binary frames
                }
            }
        }
    }
}

/// parse an HTTP request from the stream and send a canned response
async fn handle_http(mut stream: TcpStream, state: Arc<ServerState>) {
    const MAX_REQUEST_BYTES: usize = 1024 * 1024;

    let mut request_bytes = Vec::with_capacity(4096);
    let header_end = loop {
        if let Some(position) = request_bytes
            .windows(4)
            .position(|bytes| bytes == b"\r\n\r\n")
        {
            break position + 4;
        }
        if request_bytes.len() >= MAX_REQUEST_BYTES {
            tracing::warn!("mock ARI request headers exceeded limit");
            return;
        }

        let mut chunk = [0_u8; 4096];
        match stream.read(&mut chunk).await {
            Ok(0) => return,
            Ok(read) => request_bytes.extend_from_slice(&chunk[..read]),
            Err(error) => {
                tracing::warn!(error = %error, "mock ARI http read failed");
                return;
            }
        }
    };

    let headers_text = String::from_utf8_lossy(&request_bytes[..header_end]);
    let mut lines = headers_text.split("\r\n");
    let request_line = lines.next().unwrap_or_default();
    let mut request_parts = request_line.splitn(3, ' ');
    let method = request_parts.next().unwrap_or_default().to_uppercase();
    let path = request_parts.next().unwrap_or_default().to_owned();
    let mut content_length = 0_usize;
    let mut authorization = None;
    let mut content_type = None;
    for line in lines.filter(|line| !line.is_empty()) {
        let Some((name, value)) = line.split_once(':') else {
            continue;
        };
        match name.trim().to_ascii_lowercase().as_str() {
            "content-length" => {
                content_length = value.trim().parse().unwrap_or_default();
            }
            "authorization" => authorization = Some(value.trim().to_owned()),
            "content-type" => content_type = Some(value.trim().to_owned()),
            _ => {}
        }
    }

    let total_length = match header_end.checked_add(content_length) {
        Some(length) if length <= MAX_REQUEST_BYTES => length,
        _ => {
            tracing::warn!(content_length, "mock ARI request body exceeded limit");
            return;
        }
    };
    while request_bytes.len() < total_length {
        let mut chunk = [0_u8; 4096];
        match stream.read(&mut chunk).await {
            Ok(0) => return,
            Ok(read) => request_bytes.extend_from_slice(&chunk[..read]),
            Err(error) => {
                tracing::warn!(error = %error, "mock ARI http body read failed");
                return;
            }
        }
    }
    let body = request_bytes[header_end..total_length].to_vec();
    state
        .requests
        .lock()
        .expect("mock request mutex poisoned")
        .push(MockRequest {
            method: method.clone(),
            path: path.clone(),
            content_length,
            authorization,
            content_type,
            body,
        });
    state.request_received.notify_waiters();

    let key = (method, path);
    let route = state.routes.get(&key).cloned().unwrap_or(MockRoute {
        status: 404,
        body: r#"{"message":"not found"}"#.to_string(),
        framing: ResponseFraming::Fixed,
        before_response: Vec::new(),
        response_delay: std::time::Duration::ZERO,
    });

    if !route.before_response.is_empty() {
        for message in &route.before_response {
            let _ = state.event_tx.send(message.clone());
        }
        // Give the already-connected WebSocket handler a scheduling turn so
        // the scripted pre-response frames reach the wire before HTTP reply.
        tokio::task::yield_now().await;
    }
    if !route.response_delay.is_zero() {
        tokio::time::sleep(route.response_delay).await;
    }

    let reason = status_reason(route.status);
    match route.framing {
        ResponseFraming::Fixed => {
            let response = format!(
                "HTTP/1.1 {status} {reason}\r\nContent-Type: application/json\r\nContent-Length: {length}\r\nConnection: close\r\n\r\n{body}",
                status = route.status,
                length = route.body.len(),
                body = route.body,
            );
            let _ = stream.write_all(response.as_bytes()).await;
        }
        ResponseFraming::Chunked => {
            let headers = format!(
                "HTTP/1.1 {} {}\r\nContent-Type: application/json\r\nTransfer-Encoding: chunked\r\nConnection: close\r\n\r\n",
                route.status, reason,
            );
            if stream.write_all(headers.as_bytes()).await.is_err() {
                return;
            }
            for chunk in route.body.as_bytes().chunks(7) {
                let prefix = format!("{:x}\r\n", chunk.len());
                if stream.write_all(prefix.as_bytes()).await.is_err()
                    || stream.write_all(chunk).await.is_err()
                    || stream.write_all(b"\r\n").await.is_err()
                {
                    return;
                }
            }
            let _ = stream.write_all(b"0\r\n\r\n").await;
        }
        ResponseFraming::Disconnect => {}
    }
}

fn status_reason(code: u16) -> &'static str {
    match code {
        200 => "OK",
        201 => "Created",
        302 => "Found",
        204 => "No Content",
        400 => "Bad Request",
        401 => "Unauthorized",
        404 => "Not Found",
        409 => "Conflict",
        500 => "Internal Server Error",
        _ => "Unknown",
    }
}
