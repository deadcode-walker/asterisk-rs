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
            output: vec![],
            channel_variables: HashMap::new(),
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
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert_eq!(msg.get("ActionID"), Some("42"));
        assert_eq!(msg.get("actionid"), Some("42"));
    }

    #[test]
    fn decode_command_response() {
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/1.0\r\n\
             Response: Follows\r\n\
             ActionID: 42\r\n\
             Privilege: Command\r\n\
             core show version\r\n\
             Asterisk 23.0.0\r\n\
             --END COMMAND--\r\n\
             \r\n",
        );
        let mut codec = AmiCodec::new();
        let msg = codec
            .decode(&mut buf)
            .expect("should decode")
            .expect("should have message");
        assert_eq!(msg.get("Response"), Some("Follows"));
        assert_eq!(msg.output.len(), 2);
        assert_eq!(msg.output[0], "core show version");
        assert_eq!(msg.output[1], "Asterisk 23.0.0");
    }

    #[test]
    fn decode_channel_variables() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Newchannel\r\n\
             Channel: PJSIP/100-0001\r\n\
             ChanVariable(DIALSTATUS): ANSWER\r\n\
             ChanVariable(FROM_DID): 5551234567\r\n\
             Uniqueid: 1234.5\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get_variable("DIALSTATUS"), Some("ANSWER"));
        assert_eq!(msg.get_variable("FROM_DID"), Some("5551234567"));
        // ChanVariable headers should NOT appear in regular headers
        assert!(msg.get("ChanVariable(DIALSTATUS)").is_none());
        // regular headers should still work
        assert_eq!(msg.get("Channel"), Some("PJSIP/100-0001"));
    }

    #[test]
    fn encode_channel_variables() {
        let mut codec = AmiCodec::new();
        let mut vars = HashMap::new();
        vars.insert("DIALSTATUS".to_string(), "ANSWER".to_string());
        let msg = RawAmiMessage {
            headers: vec![("Action".into(), "Originate".into())],
            output: vec![],
            channel_variables: vars,
        };
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).expect("encode should succeed");
        let encoded = String::from_utf8_lossy(&buf);
        assert!(encoded.contains("Action: Originate\r\n"));
        assert!(encoded.contains("ChanVariable(DIALSTATUS): ANSWER\r\n"));
        assert!(encoded.ends_with("\r\n\r\n") || encoded.ends_with("\r\n"));
    }

    #[test]
    fn decode_empty_channel_variable() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\n\
             ChanVariable(): \r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        // empty parens => empty string key
        assert_eq!(msg.get_variable(""), Some(""));
    }

    #[test]
    fn non_chanvariable_parens_stays_in_headers() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\n\
             ChanVariableExtra(x): y\r\n\
             ChanVariable: plain\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        // these are NOT channel variables
        assert_eq!(msg.get("ChanVariableExtra(x)"), Some("y"));
        assert_eq!(msg.get("ChanVariable"), Some("plain"));
        assert!(msg.channel_variables.is_empty());
    }

    #[test]
    fn decode_invalid_banner() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from("HTTP/1.1 200 OK\r\n");
        let err = codec.decode(&mut buf).expect_err("should reject non-AMI banner");
        let msg = err.to_string();
        assert!(msg.contains("expected AMI banner"), "error should mention banner: {msg}");
    }

    #[test]
    fn decode_empty_banner() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from("\r\n");
        let err = codec.decode(&mut buf).expect_err("empty line is not a valid banner");
        let msg = err.to_string();
        assert!(msg.contains("expected AMI banner"), "error should mention banner: {msg}");
    }

    #[test]
    fn decode_no_colon_header_treated_as_output() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Response: Follows\r\n\
             ActionID: 99\r\n\
             this line has no colon\r\n\
             another output line\r\n\
             --END COMMAND--\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.output.len(), 2);
        assert_eq!(msg.output[0], "this line has no colon");
        assert_eq!(msg.output[1], "another output line");
    }

    #[test]
    fn decode_header_with_empty_value() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\n\
             Key:\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Key"), Some(""));
    }

    #[test]
    fn decode_header_value_with_colons() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Error\r\n\
             Message: Error: something: failed\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Message"), Some("Error: something: failed"));
    }

    #[test]
    fn decode_unicode_in_headers() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\n\
             CallerIDName: José García\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("CallerIDName"), Some("José García"));
    }

    #[test]
    fn decode_just_terminator() {
        // empty message after banner → headers empty → recursive decode returns None
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from("\r\n\r\n");
        let result = codec
            .decode(&mut buf)
            .expect("decode should succeed");
        assert!(result.is_none(), "empty message should yield None");
        // buffer should be consumed
        assert!(buf.is_empty());
    }

    #[test]
    fn decode_message_at_exact_size_limit() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        // build a message that is exactly MAX_MESSAGE_SIZE bytes
        // "Key: <value>\r\n\r\n" → overhead is 4 + 2 + 2 + 2 = 10
        let overhead = b"Key: \r\n\r\n".len();
        let value_len = MAX_MESSAGE_SIZE - overhead;
        let value: String = "A".repeat(value_len);
        let raw = format!("Key: {value}\r\n\r\n");
        assert_eq!(raw.len(), MAX_MESSAGE_SIZE);
        let mut buf = BytesMut::from(raw.as_str());
        let msg = codec
            .decode(&mut buf)
            .expect("exactly at limit should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Key").expect("Key header present").len(), value_len);
    }

    #[test]
    fn decode_consecutive_terminators() {
        // first \r\n\r\n is empty → skipped, second has content
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "\r\n\r\nEvent: Ping\r\n\r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce the Ping event");
        assert_eq!(msg.get("Event"), Some("Ping"));
    }

    #[test]
    fn get_all_returns_multiple_values() {
        let msg = RawAmiMessage {
            headers: vec![
                ("Allow".into(), "GET".into()),
                ("Allow".into(), "POST".into()),
                ("Allow".into(), "DELETE".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let vals = msg.get_all("Allow");
        assert_eq!(vals, vec!["GET", "POST", "DELETE"]);
        // get() returns only the first
        assert_eq!(msg.get("Allow"), Some("GET"));
    }

    #[test]
    fn to_map_last_value_wins() {
        let msg = RawAmiMessage {
            headers: vec![
                ("Key".into(), "first".into()),
                ("Key".into(), "second".into()),
                ("Key".into(), "third".into()),
            ],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let map = msg.to_map();
        assert_eq!(map.get("Key").expect("Key should exist"), "third");
    }

    #[test]
    fn get_returns_none_for_missing_key() {
        let msg = RawAmiMessage {
            headers: vec![("Event".into(), "Test".into())],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(msg.get("NonExistent").is_none());
        assert!(msg.get_variable("missing").is_none());
        assert!(msg.get_all("nope").is_empty());
    }

    #[test]
    fn encode_empty_message() {
        let mut codec = AmiCodec::new();
        let msg = RawAmiMessage {
            headers: vec![],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        let mut buf = BytesMut::new();
        codec.encode(msg, &mut buf).expect("encode should succeed");
        // only the terminator
        assert_eq!(&buf[..], b"\r\n");
    }

    #[test]
    fn decode_banner_with_version_suffix() {
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/1.0\r\n\
             Response: Success\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Response"), Some("Success"));
    }

    #[test]
    fn decode_only_output_lines_after_banner() {
        // message with only output lines (no Key: Value) → empty headers → recurse → None
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "this has no colon\r\n\
             neither does this\r\n\
             \r\n",
        );
        let result = codec.decode(&mut buf).expect("decode should succeed");
        assert!(result.is_none(), "output-only message should yield None");
    }

    #[test]
    fn raw_message_is_response_and_event_both_false() {
        let msg = RawAmiMessage {
            headers: vec![("ActionID".into(), "42".into())],
            output: vec![],
            channel_variables: HashMap::new(),
        };
        assert!(!msg.is_response());
        assert!(!msg.is_event());
    }

    #[test]
    fn decode_very_long_header_value() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let long_val = "X".repeat(10_000);
        let raw = format!("Event: Test\r\nData: {long_val}\r\n\r\n");
        let mut buf = BytesMut::from(raw.as_str());
        let msg = codec
            .decode(&mut buf)
            .expect("long value within limit should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Data").expect("Data header").len(), 10_000);
    }

    #[test]
    fn decode_windows_line_endings() {
        // AMI uses \r\n natively — verify standard handling
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\nKey: Value\r\n\r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get("Event"), Some("Test"));
        assert_eq!(msg.get("Key"), Some("Value"));
    }

    #[test]
    fn channel_variable_with_special_chars_in_name() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Test\r\n\
             ChanVariable(SOME.VAR-NAME): hello\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.get_variable("SOME.VAR-NAME"), Some("hello"));
    }

    #[test]
    fn multiple_channel_variables() {
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(
            "Event: Newchannel\r\n\
             ChanVariable(VAR1): one\r\n\
             ChanVariable(VAR2): two\r\n\
             ChanVariable(VAR3): three\r\n\
             ChanVariable(VAR4): four\r\n\
             ChanVariable(VAR5): five\r\n\
             \r\n",
        );
        let msg = codec
            .decode(&mut buf)
            .expect("decode should succeed")
            .expect("should produce a message");
        assert_eq!(msg.channel_variables.len(), 5);
        assert_eq!(msg.get_variable("VAR1"), Some("one"));
        assert_eq!(msg.get_variable("VAR2"), Some("two"));
        assert_eq!(msg.get_variable("VAR3"), Some("three"));
        assert_eq!(msg.get_variable("VAR4"), Some("four"));
        assert_eq!(msg.get_variable("VAR5"), Some("five"));
    }

    #[test]
    fn decode_after_oversized_rejection() {
        // after an oversized error, the codec is in an undefined state —
        // verify the error itself is correct
        let mut codec = AmiCodec::new();
        codec.banner_consumed = true;
        let mut buf = BytesMut::from(vec![b'X'; MAX_MESSAGE_SIZE + 1].as_slice());
        let err = codec.decode(&mut buf).expect_err("should reject oversized");
        let msg = err.to_string();
        assert!(msg.contains("limit"), "error should mention limit: {msg}");
    }

    #[test]
    fn banner_consumed_flag_prevents_recheck() {
        // after banner is consumed, subsequent messages don't need a banner
        let mut codec = AmiCodec::new();
        let mut buf = BytesMut::from(
            "Asterisk Call Manager/6.0.0\r\n\
             Event: First\r\n\
             \r\n",
        );
        let msg1 = codec
            .decode(&mut buf)
            .expect("first decode should succeed")
            .expect("should produce first message");
        assert_eq!(msg1.get("Event"), Some("First"));
        assert!(codec.banner_consumed, "banner_consumed should be true after first decode");

        // second message: no banner prefix, just a regular message
        buf.extend_from_slice(b"Event: Second\r\n\r\n");
        let msg2 = codec
            .decode(&mut buf)
            .expect("second decode should succeed")
            .expect("should produce second message");
        assert_eq!(msg2.get("Event"), Some("Second"));
    }
}
