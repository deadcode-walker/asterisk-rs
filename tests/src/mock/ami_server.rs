use std::net::SocketAddr;
use std::sync::Arc;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::{TcpListener, TcpStream};
use tokio::sync::Notify;

pub struct MockAmiServer {
    listener: TcpListener,
    addr: SocketAddr,
}

impl MockAmiServer {
    pub async fn start() -> Self {
        let listener = TcpListener::bind("127.0.0.1:0")
            .await
            .expect("mock ami: failed to bind");
        let addr = listener
            .local_addr()
            .expect("mock ami: failed to get local addr");
        Self { listener, addr }
    }

    #[allow(dead_code)]
    pub fn addr(&self) -> SocketAddr {
        self.addr
    }

    pub fn port(&self) -> u16 {
        self.addr.port()
    }

    /// accept a single connection, spawning handler as a background task.
    /// returns a JoinHandle so tests can await completion.
    pub fn accept_one<F, Fut>(self, handler: F) -> tokio::task::JoinHandle<()>
    where
        F: FnOnce(MockAmiConnection) -> Fut + Send + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        tokio::spawn(async move {
            let (stream, _peer) = self
                .listener
                .accept()
                .await
                .expect("mock ami: failed to accept");
            let conn = MockAmiConnection::new(stream).await;
            handler(conn).await;
        })
    }

    /// accept connections in a loop, calling handler for each.
    /// handler receives (connection, connection_index starting at 0).
    /// the accept loop runs until the JoinHandle is aborted or the listener errors.
    pub fn accept_loop<F, Fut>(self, handler: F) -> (tokio::task::JoinHandle<()>, Arc<Notify>)
    where
        F: Fn(MockAmiConnection, usize) -> Fut + Send + Sync + 'static,
        Fut: std::future::Future<Output = ()> + Send,
    {
        let handler = Arc::new(handler);
        let ready = Arc::new(Notify::new());
        let ready_clone = ready.clone();
        let handle = tokio::spawn(async move {
            let mut index = 0;
            loop {
                // signal that we're ready to accept
                ready_clone.notify_waiters();
                match self.listener.accept().await {
                    Ok((stream, _peer)) => {
                        let conn = MockAmiConnection::new(stream).await;
                        let h = handler.clone();
                        let i = index;
                        index += 1;
                        h(conn, i).await;
                    }
                    Err(_) => break,
                }
            }
        });
        (handle, ready)
    }
}

/// wraps a TCP stream for mock AMI server-side protocol handling
pub struct MockAmiConnection {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl MockAmiConnection {
    async fn new(stream: TcpStream) -> Self {
        let (read, mut write) = stream.into_split();
        // send banner immediately — client codec expects it before any messages
        write
            .write_all(b"Asterisk Call Manager/6.0.0\r\n")
            .await
            .expect("mock ami: failed to send banner");
        Self {
            reader: BufReader::new(read),
            writer: write,
        }
    }

    /// create from a raw TCP stream without sending a banner.
    /// useful for testing non-Asterisk server scenarios.
    pub async fn new_no_banner(stream: TcpStream) -> Self {
        let (read, write) = stream.into_split();
        Self {
            reader: BufReader::new(read),
            writer: write,
        }
    }

    /// read the next AMI message (blocks until \r\n\r\n).
    /// returns headers as Vec<(String, String)>, or None on EOF.
    pub async fn read_message(&mut self) -> Option<Vec<(String, String)>> {
        let mut headers = Vec::new();
        loop {
            let mut line = String::new();
            let n = self.reader.read_line(&mut line).await.ok()?;
            if n == 0 {
                return None; // eof
            }
            let trimmed = line.trim_end_matches("\r\n").trim_end_matches('\n');
            if trimmed.is_empty() {
                if headers.is_empty() {
                    continue; // skip blank lines between messages
                }
                return Some(headers);
            }
            if let Some((k, v)) = trimmed.split_once(':') {
                headers.push((k.trim().to_string(), v.trim().to_string()));
            }
        }
    }

    /// send an AMI response message
    pub async fn send_message(&mut self, headers: &[(&str, &str)]) {
        let mut buf = String::new();
        for (k, v) in headers {
            buf.push_str(&format!("{k}: {v}\r\n"));
        }
        buf.push_str("\r\n"); // message terminator
        self.writer
            .write_all(buf.as_bytes())
            .await
            .expect("mock ami: failed to send message");
    }

    /// send raw bytes (for command output / non-standard framing)
    pub async fn send_raw(&mut self, data: &[u8]) {
        self.writer
            .write_all(data)
            .await
            .expect("mock ami: failed to send raw");
    }
}

/// find a header value by key (case-insensitive)
pub fn get_header<'a>(headers: &'a [(String, String)], key: &str) -> Option<&'a str> {
    headers
        .iter()
        .find(|(k, _)| k.eq_ignore_ascii_case(key))
        .map(|(_, v)| v.as_str())
}

/// handle the standard login sequence: challenge + md5 login.
/// used by most tests that need an authenticated connection.
pub async fn handle_login(conn: &mut MockAmiConnection) {
    // read Challenge action
    let msg = conn
        .read_message()
        .await
        .expect("should receive challenge action");
    let action_id = get_header(&msg, "ActionID")
        .expect("challenge should have ActionID")
        .to_string();

    // respond with a challenge
    conn.send_message(&[
        ("Response", "Success"),
        ("ActionID", &action_id),
        ("Challenge", "12345678"),
    ])
    .await;

    // read Login action (md5)
    let msg = conn
        .read_message()
        .await
        .expect("should receive login action");
    let action_id = get_header(&msg, "ActionID")
        .expect("login should have ActionID")
        .to_string();

    // respond success
    conn.send_message(&[
        ("Response", "Success"),
        ("ActionID", &action_id),
        ("Message", "Authentication accepted"),
    ])
    .await;
}

/// reject the login sequence: respond Error to both challenge and plaintext login.
/// used to test login failure / reconnect behavior.
pub async fn handle_login_reject(conn: &mut MockAmiConnection) {
    // read Challenge action
    let msg = conn
        .read_message()
        .await
        .expect("should receive challenge action");
    let action_id = get_header(&msg, "ActionID")
        .expect("challenge should have ActionID")
        .to_string();

    conn.send_message(&[
        ("Response", "Error"),
        ("ActionID", &action_id),
        ("Message", "Challenge not supported"),
    ])
    .await;

    // read plaintext Login action
    let msg = conn
        .read_message()
        .await
        .expect("should receive login action");
    let action_id = get_header(&msg, "ActionID")
        .expect("login should have ActionID")
        .to_string();

    conn.send_message(&[
        ("Response", "Error"),
        ("ActionID", &action_id),
        ("Message", "Authentication failed"),
    ])
    .await;
}
