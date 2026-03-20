//! AMI wire protocol codec.
//!
//! Handles framing of AMI's `Key: Value\r\n` line protocol with
//! `\r\n\r\n` message termination.

use bytes::{Buf, BytesMut};
use std::collections::HashMap;
use tokio_util::codec::{Decoder, Encoder};

use crate::error::AmiError;

/// maximum size of a single AMI message (64 KiB)
const MAX_MESSAGE_SIZE: usize = 64 * 1024;

/// raw AMI message as parsed from the wire
#[derive(Debug, Clone, PartialEq)]
pub struct RawAmiMessage {
    /// ordered key-value headers
    pub headers: Vec<(String, String)>,
    /// command output lines (for Response: Follows)
    pub output: Vec<String>,
    /// channel variables extracted from ChanVariable(name) headers
    pub channel_variables: HashMap<String, String>,
}

impl RawAmiMessage {
    /// get the first value for a given key (case-insensitive)
    pub fn get(&self, key: &str) -> Option<&str> {
        self.headers
            .iter()
            .find(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
    }

    /// get all values for a given key (case-insensitive)
    pub fn get_all(&self, key: &str) -> Vec<&str> {
        self.headers
            .iter()
            .filter(|(k, _)| k.eq_ignore_ascii_case(key))
            .map(|(_, v)| v.as_str())
            .collect()
    }

    /// check if this is a response message
    pub fn is_response(&self) -> bool {
        self.get("Response").is_some()
    }

    /// check if this is an event message
    pub fn is_event(&self) -> bool {
        self.get("Event").is_some()
    }

    /// get a channel variable by name
    pub fn get_variable(&self, name: &str) -> Option<&str> {
        self.channel_variables.get(name).map(|s| s.as_str())
    }

    /// convert headers to a HashMap (last value wins for duplicates)
    pub fn to_map(&self) -> HashMap<String, String> {
        self.headers
            .iter()
            .map(|(k, v)| (k.clone(), v.clone()))
            .collect()
    }
}

/// codec for AMI's line-based protocol
pub struct AmiCodec {
    /// tracks whether we've consumed the initial banner line
    banner_consumed: bool,
}

impl AmiCodec {
    pub fn new() -> Self {
        Self {
            banner_consumed: false,
        }
    }
}

impl Default for AmiCodec {
    fn default() -> Self {
        Self::new()
    }
}

impl Decoder for AmiCodec {
    type Item = RawAmiMessage;
    type Error = AmiError;

    fn decode(&mut self, src: &mut BytesMut) -> Result<Option<Self::Item>, Self::Error> {
        // guard against oversized messages
        if src.len() > MAX_MESSAGE_SIZE {
            return Err(AmiError::Protocol(
                asterisk_rs_core::error::ProtocolError::MalformedMessage {
                    details: format!("message exceeds {} byte limit", MAX_MESSAGE_SIZE),
                },
            ));
        }

        // consume the banner line on first message
        if !self.banner_consumed {
            if let Some(pos) = find_crlf(src) {
                let line = &src[..pos];
                // validate it looks like an AMI banner
                if !line.starts_with(b"Asterisk Call Manager") {
                    return Err(AmiError::Protocol(
                        asterisk_rs_core::error::ProtocolError::MalformedMessage {
                            details: format!(
                                "expected AMI banner, got: {}",
                                String::from_utf8_lossy(line)
                            ),
                        },
                    ));
                }
                src.advance(pos + 2); // skip line + \r\n
                self.banner_consumed = true;
            } else {
                return Ok(None); // need more data
            }
        }

        // look for message terminator: \r\n\r\n
        let end_pos = match find_double_crlf(src) {
            Some(pos) => pos,
            None => return Ok(None), // need more data
        };

        // extract the message bytes (not including the final \r\n\r\n)
        let message_bytes = &src[..end_pos];
        let mut headers = Vec::new();
        let mut output = Vec::new();
        let mut channel_variables = HashMap::new();

        for line in message_bytes.split(|&b| b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
                continue;
            }

            // skip the END COMMAND marker
            if line == b"--END COMMAND--" {
                continue;
            }

            // split on first ':'
            if let Some(colon_pos) = line.iter().position(|&b| b == b':') {
                let key = String::from_utf8_lossy(&line[..colon_pos])
                    .trim()
                    .to_string();
                let value_start = colon_pos + 1;
                let value = if value_start < line.len() {
                    String::from_utf8_lossy(&line[value_start..])
                        .trim()
                        .to_string()
                } else {
                    String::new()
                };
                if key.starts_with("ChanVariable(") && key.ends_with(')') {
                    let var_name = &key["ChanVariable(".len()..key.len() - 1];
                    channel_variables.insert(var_name.to_string(), value);
                } else {
                    headers.push((key, value));
                }
            } else {
                // command output line (e.g., Response: Follows body)
                output.push(String::from_utf8_lossy(line).to_string());
            }
        }

        // advance past the message + terminator (\r\n\r\n = 4 bytes)
        src.advance(end_pos + 4);

        if headers.is_empty() {
            // empty message, try next
            return self.decode(src);
        }

        Ok(Some(RawAmiMessage {
            headers,
            output,
            channel_variables,
        }))
    }
}

impl Encoder<RawAmiMessage> for AmiCodec {
    type Error = AmiError;

    fn encode(&mut self, item: RawAmiMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        // reject any header key or value containing CR or LF — either can inject
        // extra AMI headers or split the frame boundary
        let contains_line_terminator = |s: &str| s.bytes().any(|b| b == b'\r' || b == b'\n');
        for (key, value) in &item.headers {
            if contains_line_terminator(key) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: format!("header key contains illegal line terminator: {:?}", key),
                    },
                ));
            }
            if contains_line_terminator(value) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: "header value contains illegal line terminator".to_owned(),
                    },
                ));
            }
        }
        for (name, value) in &item.channel_variables {
            if contains_line_terminator(name) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: format!(
                            "channel variable name contains illegal line terminator: {:?}",
                            name
                        ),
                    },
                ));
            }
            if contains_line_terminator(value) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: "channel variable value contains illegal line terminator"
                            .to_owned(),
                    },
                ));
            }
        }

        for (key, value) in &item.headers {
            dst.extend_from_slice(key.as_bytes());
            dst.extend_from_slice(b": ");
            dst.extend_from_slice(value.as_bytes());
            dst.extend_from_slice(b"\r\n");
        }
        for (name, value) in &item.channel_variables {
            dst.extend_from_slice(format!("ChanVariable({})", name).as_bytes());
            dst.extend_from_slice(b": ");
            dst.extend_from_slice(value.as_bytes());
            dst.extend_from_slice(b"\r\n");
        }
        dst.extend_from_slice(b"\r\n"); // message terminator
        Ok(())
    }
}

/// find the position of the first \r\n in the buffer
fn find_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(2).position(|w| w == b"\r\n")
}

/// find the position of \r\n\r\n (returns position of first \r)
fn find_double_crlf(buf: &[u8]) -> Option<usize> {
    buf.windows(4).position(|w| w == b"\r\n\r\n")
}
