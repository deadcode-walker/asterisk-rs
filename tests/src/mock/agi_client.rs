use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpStream;

/// simulates Asterisk connecting to a FastAGI server
pub struct MockAgiClient {
    reader: BufReader<tokio::net::tcp::OwnedReadHalf>,
    writer: tokio::net::tcp::OwnedWriteHalf,
}

impl MockAgiClient {
    /// connect to the AGI server and send environment variables
    pub async fn connect(addr: SocketAddr, env_vars: &[(&str, &str)]) -> Self {
        let stream = TcpStream::connect(addr)
            .await
            .expect("mock agi: failed to connect");
        let (read, mut write) = stream.into_split();

        // send AGI environment variables
        for (key, value) in env_vars {
            let line = format!("agi_{key}: {value}\n");
            write
                .write_all(line.as_bytes())
                .await
                .expect("mock agi: failed to send env var");
        }
        // blank line terminates environment block
        write
            .write_all(b"\n")
            .await
            .expect("mock agi: failed to send blank line");

        Self {
            reader: BufReader::new(read),
            writer: write,
        }
    }

    /// read the next command from the AGI server
    pub async fn read_command(&mut self) -> Option<String> {
        let mut line = String::new();
        let n = self.reader.read_line(&mut line).await.ok()?;
        if n == 0 {
            return None;
        }
        Some(line.trim().to_string())
    }

    /// send a response (e.g., "200 result=0")
    pub async fn send_response(&mut self, response: &str) {
        let msg = format!("{response}\n");
        self.writer
            .write_all(msg.as_bytes())
            .await
            .expect("mock agi: failed to send response");
    }

    /// send a 200 success response
    pub async fn send_success(&mut self, result: i32) {
        self.send_response(&format!("200 result={result}")).await;
    }

    /// send a hangup response (511)
    pub async fn send_hangup(&mut self) {
        self.send_response("511 result=-1 Channel gone").await;
    }
}

/// standard AGI environment variables matching a typical Asterisk session
pub fn standard_env() -> Vec<(&'static str, &'static str)> {
    vec![
        ("request", "agi://localhost/test"),
        ("channel", "SIP/100-00000001"),
        ("language", "en"),
        ("type", "SIP"),
        ("uniqueid", "1234567890.1"),
        ("version", "22.0.0"),
        ("callerid", "100"),
        ("calleridname", "Test User"),
        ("callingpres", "0"),
        ("callingani2", "0"),
        ("callington", "0"),
        ("callingtns", "0"),
        ("dnid", "200"),
        ("rdnis", "unknown"),
        ("context", "default"),
        ("extension", "200"),
        ("priority", "1"),
        ("enhanced", "0.0"),
        ("accountcode", ""),
        ("threadid", "1234"),
        ("network", "yes"),
        ("network_script", "test"),
    ]
}

/// bind to port 0 and return the assigned port, releasing it immediately.
/// the TOCTOU window is tiny and acceptable for tests.
pub async fn free_port() -> u16 {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0")
        .await
        .expect("failed to bind for free port");
    listener
        .local_addr()
        .expect("failed to get local addr")
        .port()
}
