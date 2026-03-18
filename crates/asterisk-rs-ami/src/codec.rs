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

        for line in message_bytes.split(|&b| b == b'\n') {
            let line = line.strip_suffix(b"\r").unwrap_or(line);
            if line.is_empty() {
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
                headers.push((key, value));
            }
            // lines without ':' are silently skipped (e.g., command output in Response: Follows)
        }

        // advance past the message + terminator (\r\n\r\n = 4 bytes)
        src.advance(end_pos + 4);

        if headers.is_empty() {
            // empty message, try next
            return self.decode(src);
        }

        Ok(Some(RawAmiMessage { headers }))
    }
}

impl Encoder<RawAmiMessage> for AmiCodec {
    type Error = AmiError;

    fn encode(&mut self, item: RawAmiMessage, dst: &mut BytesMut) -> Result<(), Self::Error> {
        for (key, value) in &item.headers {
            dst.extend_from_slice(key.as_bytes());
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

#[cfg(test)]
mod tests {
    use super::*;
    use bytes::BytesMut;

    #[test]
    fn decode_banner_and_response() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/6.0.0\r\n\
             Response: Success\r\n\
             ActionID: 1\r\n\
             Message: Authentication accepted\r\n\
             \r\n",
        );

        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Response"), Some("Success"));
        assert_eq!(msg.get("ActionID"), Some("1"));
        assert_eq!(msg.get("Message"), Some("Authentication accepted"));
        assert!(msg.is_response());
        assert!(!msg.is_event());
    }

    #[test]
    fn decode_event() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/6.0.0\r\n\
             Event: FullyBooted\r\n\
             Privilege: system,all\r\n\
             Status: Fully Booted\r\n\
             \r\n",
        );

        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Event"), Some("FullyBooted"));
        assert!(msg.is_event());
    }

    #[test]
    fn decode_partial_message() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/6.0.0\r\n\
             Response: Success\r\n\
             ActionID: 1\r\n",
        );
        // no terminator yet
        assert!(codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .is_none());

        // now add the terminator
        buf.extend_from_slice(b"\r\n");
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Response"), Some("Success"));
    }

    #[test]
    fn encode_action() {
        let mut codec = AmiCodec::new();
        let msg = RawAmiMessage {
            headers: vec![
                ("Action".into(), "Login".into()),
                ("Username".into(), "admin".into()),
                ("Secret".into(), "password".into()),
                ("ActionID".into(), "1".into()),
            ],
        };
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).expect("encode should succeed");
        assert_eq!(
            &buf[..],
            b"Action: Login\r\nUsername: admin\r\nSecret: password\r\nActionID: 1\r\n\r\n"
        );
    }

    #[test]
    fn reject_oversized_message() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(vec![b'A'; MAX_MESSAGE_SIZE + 1].as_slice());
        assert!(codec.decode(&mut buf).is_err());
    }

    #[test]
    fn decode_multiple_messages() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/6.0.0\r\n\
             Response: Success\r\n\
             ActionID: 1\r\n\
             \r\n\
             Event: Hangup\r\n\
             Channel: SIP/100\r\n\
             \r\n",
        );

        let msg1 = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce first message");
        assert!(msg1.is_response());

        let msg2 = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce second message");
        assert!(msg2.is_event());
        assert_eq!(msg2.get("Event"), Some("Hangup"));
    }

    #[test]
    fn case_insensitive_header_lookup() {
        let msg = RawAmiMessage {
            headers: vec![("actionid".into(), "42".into())],
        };
        assert_eq!(msg.get("ActionID"), Some("42"));
        assert_eq!(msg.get("actionid"), Some("42"));
    }
}
