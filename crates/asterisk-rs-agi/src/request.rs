use std::collections::HashMap;
use std::net::SocketAddr;

use tokio::io::{AsyncBufReadExt, AsyncReadExt};

const MAX_PRELUDE_LINE_BYTES: usize = 8 * 1024;
const MAX_PRELUDE_BYTES: usize = 64 * 1024;
const MAX_PRELUDE_VARIABLES: usize = 128;

/// parsed AGI request environment sent by asterisk on connection
#[derive(Debug, Clone)]
pub struct AgiRequest {
    /// all agi_* variables as key-value pairs (key without "agi_" prefix)
    variables: HashMap<String, String>,
    peer_addr: Option<SocketAddr>,
}

impl AgiRequest {
    /// parse agi environment variables from the initial connection
    ///
    /// reads newline-terminated variables through the required empty `LF` or
    /// `CRLF` line. Each line must contain a name and `:` separator. The parser
    /// strips the `agi_` key prefix and one separator space while preserving all
    /// other value whitespace. Unterminated, malformed, or oversized preludes
    /// are rejected.
    pub async fn parse_from_reader<R: tokio::io::AsyncBufRead + Unpin>(
        reader: &mut R,
    ) -> crate::error::Result<Self> {
        let mut variables = HashMap::new();
        let mut line = String::new();
        let mut total_bytes = 0usize;

        loop {
            line.clear();
            // limit bytes read per line to prevent OOM from a malicious client
            // sending an unbounded line without a newline
            let bytes_read = (&mut *reader)
                .take((MAX_PRELUDE_LINE_BYTES + 1) as u64)
                .read_line(&mut line)
                .await?;

            total_bytes = total_bytes.saturating_add(bytes_read);
            if total_bytes > MAX_PRELUDE_BYTES {
                return Err(invalid_request("AGI prelude exceeds 65536 bytes"));
            }

            if bytes_read == 0 {
                return Err(invalid_request(
                    "AGI prelude ended before its blank-line terminator",
                ));
            }

            if line.len() > MAX_PRELUDE_LINE_BYTES {
                return Err(invalid_request("AGI prelude line exceeds 8192 bytes"));
            }
            if !line.ends_with('\n') {
                return Err(invalid_request("AGI prelude line ended without a newline"));
            }

            let content = line.strip_suffix('\n').expect("line ending checked above");
            let content = content.strip_suffix('\r').unwrap_or(content);
            if content.is_empty() {
                break;
            }
            if content.contains('\r') || content.contains('\0') {
                return Err(invalid_request(
                    "AGI prelude line contains a forbidden control character",
                ));
            }

            let (key, value) = content
                .split_once(':')
                .ok_or_else(|| invalid_request("AGI prelude line is missing ':'"))?;
            let key = key.trim();
            if key.is_empty() {
                return Err(invalid_request("AGI prelude variable name is empty"));
            }

            // strip one protocol separator space but preserve value whitespace
            let value = value.strip_prefix(' ').unwrap_or(value);

            // strip the agi_ prefix from keys
            let key = key.strip_prefix("agi_").unwrap_or(key);
            if key.is_empty() {
                return Err(invalid_request("AGI prelude variable name is empty"));
            }
            if !variables.contains_key(key) && variables.len() >= MAX_PRELUDE_VARIABLES {
                return Err(invalid_request("AGI prelude exceeds 128 variables"));
            }
            variables.insert(key.to_owned(), value.to_owned());
        }

        Ok(Self {
            variables,
            peer_addr: None,
        })
    }

    pub(crate) fn set_peer_addr(&mut self, peer_addr: SocketAddr) {
        self.peer_addr = Some(peer_addr);
    }

    /// return the TCP peer observed by [`crate::server::AgiServer`]
    ///
    /// requests parsed directly from a reader do not have peer metadata
    pub fn peer_addr(&self) -> Option<SocketAddr> {
        self.peer_addr
    }

    /// get the value of `agi_network`
    pub fn network(&self) -> Option<&str> {
        self.variables.get("network").map(String::as_str)
    }

    /// get the value of `agi_network_script`
    pub fn network_script(&self) -> Option<&str> {
        self.variables.get("network_script").map(String::as_str)
    }

    /// get the value of `agi_request`
    pub fn request(&self) -> Option<&str> {
        self.variables.get("request").map(String::as_str)
    }

    /// get the value of `agi_channel`
    pub fn channel(&self) -> Option<&str> {
        self.variables.get("channel").map(String::as_str)
    }

    /// get the value of `agi_language`
    pub fn language(&self) -> Option<&str> {
        self.variables.get("language").map(String::as_str)
    }

    /// get the value of `agi_type`
    pub fn channel_type(&self) -> Option<&str> {
        self.variables.get("type").map(String::as_str)
    }

    /// get the value of `agi_uniqueid`
    pub fn unique_id(&self) -> Option<&str> {
        self.variables.get("uniqueid").map(String::as_str)
    }

    /// get the value of `agi_callerid`
    pub fn caller_id(&self) -> Option<&str> {
        self.variables.get("callerid").map(String::as_str)
    }

    /// get the value of `agi_calleridname`
    pub fn caller_id_name(&self) -> Option<&str> {
        self.variables.get("calleridname").map(String::as_str)
    }

    /// get the value of `agi_context`
    pub fn context(&self) -> Option<&str> {
        self.variables.get("context").map(String::as_str)
    }

    /// get the value of `agi_extension`
    pub fn extension(&self) -> Option<&str> {
        self.variables.get("extension").map(String::as_str)
    }

    /// get the value of `agi_priority`
    pub fn priority(&self) -> Option<&str> {
        self.variables.get("priority").map(String::as_str)
    }

    /// generic accessor for any variable by key (without `agi_` prefix)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.variables.get(key).map(String::as_str)
    }
}

fn invalid_request(details: &'static str) -> crate::error::AgiError {
    crate::error::AgiError::InvalidRequest {
        details: details.to_owned(),
    }
}
