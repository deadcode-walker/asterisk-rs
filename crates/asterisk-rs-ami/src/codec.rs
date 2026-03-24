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
#[derive(Debug)]
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
        // consume the banner line on first message
        if !self.banner_consumed {
            if let Some(pos) = find_crlf(src) {
                let line = &src[..pos];
                // validate it looks like an AMI banner
                if !line.starts_with(b"Asterisk Call Manager") {
                    let preview = String::from_utf8_lossy(&line[..line.len().min(64)]);
                    return Err(AmiError::Protocol(
                        asterisk_rs_core::error::ProtocolError::MalformedMessage {
                            details: format!("expected AMI banner, got: {}", preview),
                        },
                    ));
                }
                src.advance(pos + 2); // skip line + \r\n
                self.banner_consumed = true;
            } else {
                return Ok(None); // need more data
            }
        }

        // AMI Response: Follows frames embed output lines between the header
        // lines and a --END COMMAND-- marker, all terminated by \r\n\r\n.
        // the output lines lack a colon-separated key, so the line parser
        // already puts them in `output`. the only framing concern is that
        // we must not accept a \r\n\r\n that appears before the end marker.
        const END_MARKER: &[u8] = b"--END COMMAND--";

        // loop to skip empty frames instead of recursing
        loop {
            let first_blank = match find_double_crlf(src) {
                Some(pos) => pos,
                None => return Ok(None),
            };

            // peek: does this frame contain a Follows header?
            // if so, the real terminator is \r\n\r\n *after* --END COMMAND--
            let frame_end = if is_follows_response(&src[..first_blank]) {
                // the marker may appear after the first \r\n\r\n because the
                // output body can contain blank lines in some edge cases.
                // scan the entire buffer for --END COMMAND--\r\n\r\n
                match find_subsequence(src, END_MARKER) {
                    Some(marker_pos) => {
                        let after_marker = marker_pos + END_MARKER.len();
                        // expect \r\n after the marker (Asterisk always sends it)
                        if src.len() < after_marker + 2 {
                            return Ok(None);
                        }
                        // then look for \r\n\r\n immediately after the marker line
                        if &src[after_marker..after_marker + 2] != b"\r\n" {
                            return Ok(None);
                        }
                        // frame ends after marker + \r\n
                        after_marker + 2
                    }
                    None => return Ok(None),
                }
            } else {
                // regular message: frame ends at first \r\n\r\n + 4
                first_blank + 4
            };

            // size check on the individual message, not the whole buffer
            if frame_end > MAX_MESSAGE_SIZE {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: format!("message exceeds {} byte limit", MAX_MESSAGE_SIZE),
                    },
                ));
            }

            // parse all lines in the frame: key:value pairs go to headers,
            // everything else goes to output (command body for Response: Follows)
            let message_bytes = &src[..frame_end];
            let mut headers = Vec::new();
            let mut output = Vec::new();
            let mut channel_variables = HashMap::new();

            for line in message_bytes.split(|&b| b == b'\n') {
                let line = line.strip_suffix(b"\r").unwrap_or(line);
                if line.is_empty() {
                    continue;
                }
                if line == END_MARKER {
                    continue;
                }
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
                    if let Some(var_name) = key
                        .strip_prefix("ChanVariable(")
                        .and_then(|s| s.strip_suffix(')'))
                    {
                        channel_variables.insert(var_name.to_string(), value);
                    } else {
                        headers.push((key, value));
                    }
                } else {
                    // non-key-value line: command output
                    output.push(String::from_utf8_lossy(line).into_owned());
                }
            }

            src.advance(frame_end);

            if headers.is_empty() {
                // empty frame, skip and try next
                continue;
            }

            return Ok(Some(RawAmiMessage {
                headers,
                output,
                channel_variables,
            }));
        }
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
            dst.extend_from_slice(b"ChanVariable(");
            dst.extend_from_slice(name.as_bytes());
            dst.extend_from_slice(b"): ");
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

/// returns true if the header block contains a `Response: Follows` header,
/// tolerating optional whitespace after the colon (e.g. `Response:Follows`)
fn is_follows_response(header_bytes: &[u8]) -> bool {
    header_bytes.split(|&b| b == b'\n').any(|line| {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if let Some(colon_pos) = line.iter().position(|&b| b == b':') {
            let key = &line[..colon_pos];
            let value = &line[colon_pos + 1..];
            let value_trimmed = value.strip_prefix(b" ").unwrap_or(value);
            key.eq_ignore_ascii_case(b"response") && value_trimmed.eq_ignore_ascii_case(b"follows")
        } else {
            false
        }
    })
}

/// find the starting position of `needle` in `haystack`
fn find_subsequence(haystack: &[u8], needle: &[u8]) -> Option<usize> {
    haystack.windows(needle.len()).position(|w| w == needle)
}
