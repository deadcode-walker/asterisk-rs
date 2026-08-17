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

/// maximum number of headers in a single AMI message
///
/// prevents excessive allocation from a malicious or misbehaving server
/// sending many tiny headers within the byte limit
const MAX_HEADERS: usize = 512;

/// raw AMI message as parsed from the wire
#[derive(Clone, PartialEq)]
pub struct RawAmiMessage {
    /// ordered key-value headers
    pub headers: Vec<(String, String)>,
    /// command output lines (for Response: Follows)
    pub output: Vec<String>,
    /// channel variables extracted from ChanVariable(name) headers
    pub channel_variables: HashMap<String, String>,
}

impl std::fmt::Debug for RawAmiMessage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RawAmiMessage")
            .field("headers", &RedactedHeaderPairs(&self.headers))
            .field("output_lines", &self.output.len())
            .field(
                "output_bytes",
                &self.output.iter().map(String::len).sum::<usize>(),
            )
            .field(
                "channel_variables",
                &RedactedHeaderMap(&self.channel_variables),
            )
            .finish()
    }
}

pub(crate) const REDACTED_HEADER_VALUE: &str = "[REDACTED]";

pub(crate) fn is_sensitive_header(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|character| character.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    normalized.contains("password")
        || normalized.contains("passwd")
        || normalized.contains("secret")
        || normalized == "md5cred"
        || normalized.contains("credential")
        || normalized.contains("token")
        || normalized.contains("authorization")
        || normalized.contains("apikey")
        || normalized.contains("privatekey")
        || normalized.contains("accesskey")
        || normalized.contains("cookie")
        || normalized == "pin"
        || normalized.ends_with("pin")
        || normalized.contains("pincode")
}

pub(crate) fn redacted_header_value<'a>(key: &str, value: &'a str) -> &'a str {
    if is_sensitive_header(key) {
        REDACTED_HEADER_VALUE
    } else {
        value
    }
}

pub(crate) struct RedactedHeaderPairs<'a>(pub(crate) &'a [(String, String)]);

impl std::fmt::Debug for RedactedHeaderPairs<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_list()
            .entries(
                self.0
                    .iter()
                    .map(|(key, value)| (key, redacted_header_value(key, value))),
            )
            .finish()
    }
}

pub(crate) struct RedactedHeaderMap<'a>(pub(crate) &'a HashMap<String, String>);

impl std::fmt::Debug for RedactedHeaderMap<'_> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_map()
            .entries(
                self.0
                    .iter()
                    .map(|(key, value)| (key, redacted_header_value(key, value))),
            )
            .finish()
    }
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

    /// approximate retained bytes for bounded multi-message aggregation
    pub(crate) fn retained_size(&self) -> usize {
        self.headers
            .iter()
            .map(|(key, value)| key.len() + value.len())
            .sum::<usize>()
            + self.output.iter().map(String::len).sum::<usize>()
            + self
                .channel_variables
                .iter()
                .map(|(name, value)| name.len() + value.len())
                .sum::<usize>()
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

    /// validate a complete outbound frame before any socket write begins
    pub(crate) fn validate_outbound(item: &RawAmiMessage) -> Result<(), AmiError> {
        let contains_line_terminator = |s: &str| s.bytes().any(|b| b == b'\r' || b == b'\n');
        if item.headers.len() + item.channel_variables.len() > MAX_HEADERS {
            return Err(AmiError::Protocol(
                asterisk_rs_core::error::ProtocolError::MalformedMessage {
                    details: format!("message exceeds {} header limit", MAX_HEADERS),
                },
            ));
        }

        let mut frame_len = 2usize;
        for (key, value) in &item.headers {
            if contains_line_terminator(key) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: format!("header key contains illegal line terminator: {key:?}"),
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
            frame_len = frame_len
                .checked_add(key.len() + value.len() + 4)
                .ok_or_else(message_too_large)?;
        }
        for (name, value) in &item.channel_variables {
            if contains_line_terminator(name) {
                return Err(AmiError::Protocol(
                    asterisk_rs_core::error::ProtocolError::MalformedMessage {
                        details: format!(
                            "channel variable name contains illegal line terminator: {name:?}"
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
            frame_len = frame_len
                .checked_add(name.len() + value.len() + 18)
                .ok_or_else(message_too_large)?;
        }
        if frame_len > MAX_MESSAGE_SIZE {
            return Err(message_too_large());
        }
        Ok(())
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
                reject_oversized_incomplete(src)?;
                return Ok(None); // need more data
            }
        }

        // AMI Response: Follows frames embed output lines between the header
        // lines and a --END COMMAND-- marker, all terminated by \r\n\r\n.
        // output may use repeated Output headers or raw lines, including
        // blank and colon-bearing lines. do not accept a \r\n\r\n that
        // appears before the line-delimited end marker.
        const END_MARKER: &[u8] = b"--END COMMAND--";
        const END_SEQUENCE: &[u8] = b"\r\n--END COMMAND--\r\n\r\n";

        // loop to skip empty frames instead of recursing
        loop {
            let first_blank = match find_double_crlf(src) {
                Some(pos) => pos,
                None => {
                    reject_oversized_incomplete(src)?;
                    return Ok(None);
                }
            };

            // peek: does this frame contain a Follows header?
            // if so, the real terminator is \r\n\r\n *after* --END COMMAND--
            let frame_end = if is_follows_response(&src[..first_blank]) {
                match find_subsequence(src, END_SEQUENCE) {
                    Some(marker_pos) => marker_pos + END_SEQUENCE.len(),
                    None => {
                        reject_oversized_incomplete(src)?;
                        return Ok(None);
                    }
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

            let message_bytes = &src[..frame_end];
            let message = parse_message(
                message_bytes,
                is_follows_response(&src[..first_blank]),
                END_MARKER,
            )?;

            src.advance(frame_end);

            if message.headers.is_empty() {
                // empty frame, skip and try next
                continue;
            }

            return Ok(Some(message));
        }
    }
}

impl Encoder<RawAmiMessage> for AmiCodec {
    type Error = AmiError;

    fn encode(&mut self, item: RawAmiMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        Self::validate_outbound(&item)?;

        let mut frame = BytesMut::new();
        for (key, value) in &item.headers {
            frame.extend_from_slice(key.as_bytes());
            frame.extend_from_slice(b": ");
            frame.extend_from_slice(value.as_bytes());
            frame.extend_from_slice(b"\r\n");
        }
        for (name, value) in &item.channel_variables {
            frame.extend_from_slice(b"ChanVariable(");
            frame.extend_from_slice(name.as_bytes());
            frame.extend_from_slice(b"): ");
            frame.extend_from_slice(value.as_bytes());
            frame.extend_from_slice(b"\r\n");
        }
        frame.extend_from_slice(b"\r\n"); // message terminator
        dst.extend_from_slice(&frame);
        Ok(())
    }
}

fn parse_message(
    message_bytes: &[u8],
    follows: bool,
    end_marker: &[u8],
) -> Result<RawAmiMessage, AmiError> {
    let mut headers = Vec::new();
    let mut output = Vec::new();
    let mut channel_variables = HashMap::new();
    let mut output_started = false;

    for line in message_bytes.split(|&byte| byte == b'\n') {
        let line = line.strip_suffix(b"\r").unwrap_or(line);
        if follows && line == end_marker {
            break;
        }
        if follows {
            if let Some(value) = output_header_value(line) {
                output_started = true;
                output.push(String::from_utf8_lossy(value).into_owned());
                continue;
            }
        }
        if follows && output_started {
            output.push(String::from_utf8_lossy(line).into_owned());
            continue;
        }
        if line.is_empty() {
            if follows && !headers.is_empty() {
                output_started = true;
                output.push(String::new());
            }
            continue;
        }

        let Some(colon_pos) = line.iter().position(|&byte| byte == b':') else {
            if follows {
                output_started = true;
            }
            output.push(String::from_utf8_lossy(line).into_owned());
            continue;
        };
        let key_bytes = trim_ascii(&line[..colon_pos]);
        if follows && !is_follows_envelope_header(key_bytes) {
            output_started = true;
            output.push(String::from_utf8_lossy(line).into_owned());
            continue;
        }
        if headers.len() + channel_variables.len() >= MAX_HEADERS {
            return Err(AmiError::Protocol(
                asterisk_rs_core::error::ProtocolError::MalformedMessage {
                    details: format!("message exceeds {} header limit", MAX_HEADERS),
                },
            ));
        }
        let key = String::from_utf8_lossy(key_bytes).into_owned();
        let value = String::from_utf8_lossy(trim_ascii(&line[colon_pos + 1..])).into_owned();
        if let Some(var_name) = key
            .strip_prefix("ChanVariable(")
            .and_then(|name| name.strip_suffix(')'))
        {
            channel_variables.insert(var_name.to_owned(), value);
        } else {
            headers.push((key, value));
        }
    }

    Ok(RawAmiMessage {
        headers,
        output,
        channel_variables,
    })
}

fn output_header_value(line: &[u8]) -> Option<&[u8]> {
    let colon_pos = line.iter().position(|&byte| byte == b':')?;
    if !trim_ascii(&line[..colon_pos]).eq_ignore_ascii_case(b"Output") {
        return None;
    }
    Some(
        line[colon_pos + 1..]
            .strip_prefix(b" ")
            .unwrap_or(&line[colon_pos + 1..]),
    )
}

fn is_follows_envelope_header(key: &[u8]) -> bool {
    [
        b"Response".as_slice(),
        b"ActionID".as_slice(),
        b"Privilege".as_slice(),
        b"Message".as_slice(),
        b"EventList".as_slice(),
        b"Timestamp".as_slice(),
        b"Server".as_slice(),
    ]
    .iter()
    .any(|candidate| key.eq_ignore_ascii_case(candidate))
}

fn trim_ascii(mut value: &[u8]) -> &[u8] {
    while value.first().is_some_and(u8::is_ascii_whitespace) {
        value = &value[1..];
    }
    while value.last().is_some_and(u8::is_ascii_whitespace) {
        value = &value[..value.len() - 1];
    }
    value
}

fn reject_oversized_incomplete(src: &BytesMut) -> Result<(), AmiError> {
    if src.len() > MAX_MESSAGE_SIZE {
        return Err(message_too_large());
    }
    Ok(())
}

fn message_too_large() -> AmiError {
    AmiError::Protocol(asterisk_rs_core::error::ProtocolError::MalformedMessage {
        details: format!("message exceeds {} byte limit", MAX_MESSAGE_SIZE),
    })
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
