#![allow(clippy::unwrap_used)]

use asterisk_rs_ami::codec::{AmiCodec, RawAmiMessage};
use bytes::BytesMut;
use std::collections::HashMap;
use tokio_util::codec::{Decoder, Encoder};

/// helper: prefix raw message bytes with a banner so the codec consumes it,
/// avoiding private `banner_consumed` field access
fn with_banner(msg: &str) -> String {
    format!("Asterisk Call Manager/6.0.0\r\n{msg}")
}

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
    // feed banner first so codec advances past it
    let banner = b"Asterisk Call Manager/6.0.0\r\n";
    // build a message that exceeds 64KB when terminated
    let oversized_value = "X".repeat(64 * 1024);
    let msg = format!("Response: Success\r\nData: {}\r\n\r\n", oversized_value);
    let mut buf = BytesMut::with_capacity(banner.len() + msg.len());
    buf.extend_from_slice(banner);
    buf.extend_from_slice(msg.as_bytes());
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
    let raw = with_banner(
        "Event: Newchannel\r\n\
         Channel: PJSIP/100-0001\r\n\
         ChanVariable(DIALSTATUS): ANSWER\r\n\
         ChanVariable(FROM_DID): 5551234567\r\n\
         Uniqueid: 1234.5\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let raw = with_banner(
        "Event: Test\r\n\
         ChanVariable(): \r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let raw = with_banner(
        "Event: Test\r\n\
         ChanVariableExtra(x): y\r\n\
         ChanVariable: plain\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let err = codec
        .decode(&mut buf)
        .expect_err("should reject non-AMI banner");
    let msg = err.to_string();
    assert!(
        msg.contains("expected AMI banner"),
        "error should mention banner: {msg}"
    );
}

#[test]
fn decode_empty_banner() {
    let mut codec = AmiCodec::new();
    let mut buf = BytesMut::from("\r\n");
    let err = codec
        .decode(&mut buf)
        .expect_err("empty line is not a valid banner");
    let msg = err.to_string();
    assert!(
        msg.contains("expected AMI banner"),
        "error should mention banner: {msg}"
    );
}

#[test]
fn decode_no_colon_header_treated_as_output() {
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Response: Follows\r\n\
         ActionID: 99\r\n\
         this line has no colon\r\n\
         another output line\r\n\
         --END COMMAND--\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let raw = with_banner(
        "Event: Test\r\n\
         Key:\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Key"), Some(""));
}

#[test]
fn decode_header_value_with_colons() {
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Event: Error\r\n\
         Message: Error: something: failed\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Message"), Some("Error: something: failed"));
}

#[test]
fn decode_unicode_in_headers() {
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Event: Test\r\n\
         CallerIDName: José García\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("CallerIDName"), Some("José García"));
}

#[test]
fn decode_just_terminator() {
    // empty message after banner -> headers empty -> recursive decode returns None
    let mut codec = AmiCodec::new();
    let raw = with_banner("\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
    let result = codec.decode(&mut buf).expect("decode should succeed");
    assert!(result.is_none(), "empty message should yield None");
    // buffer should be consumed
    assert!(buf.is_empty());
}

#[test]
fn decode_message_at_exact_size_limit() {
    let mut codec = AmiCodec::new();
    // feed the banner first so the codec moves past it
    let mut buf = BytesMut::from("Asterisk Call Manager/6.0.0\r\n");
    codec
        .decode(&mut buf)
        .expect("banner decode should succeed");

    // build a message that is exactly 64 KiB
    let max_message_size: usize = 64 * 1024;
    let overhead = b"Key: \r\n\r\n".len();
    let value_len = max_message_size - overhead;
    let value: String = "A".repeat(value_len);
    let raw = format!("Key: {value}\r\n\r\n");
    assert_eq!(raw.len(), max_message_size);
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("exactly at limit should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Key").expect("Key header present").len(), value_len);
}

#[test]
fn decode_consecutive_terminators() {
    // first \r\n\r\n is empty -> skipped, second has content
    let mut codec = AmiCodec::new();
    let raw = with_banner("\r\n\r\nEvent: Ping\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
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
    // message with only output lines (no Key: Value) -> empty headers -> recurse -> None
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "this has no colon\r\n\
         neither does this\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let long_val = "X".repeat(10_000);
    let raw = with_banner(&format!("Event: Test\r\nData: {long_val}\r\n\r\n"));
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
    let raw = with_banner("Event: Test\r\nKey: Value\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
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
    let raw = with_banner(
        "Event: Test\r\n\
         ChanVariable(SOME.VAR-NAME): hello\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get_variable("SOME.VAR-NAME"), Some("hello"));
}

#[test]
fn multiple_channel_variables() {
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Event: Newchannel\r\n\
         ChanVariable(VAR1): one\r\n\
         ChanVariable(VAR2): two\r\n\
         ChanVariable(VAR3): three\r\n\
         ChanVariable(VAR4): four\r\n\
         ChanVariable(VAR5): five\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
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
    let banner = b"Asterisk Call Manager/6.0.0\r\n";
    let oversized_value = "X".repeat(64 * 1024);
    let msg = format!("Response: Success\r\nData: {}\r\n\r\n", oversized_value);
    let mut buf = BytesMut::with_capacity(banner.len() + msg.len());
    buf.extend_from_slice(banner);
    buf.extend_from_slice(msg.as_bytes());
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

    // second message: no banner prefix, just a regular message
    buf.extend_from_slice(b"Event: Second\r\n\r\n");
    let msg2 = codec
        .decode(&mut buf)
        .expect("second decode should succeed")
        .expect("should produce second message");
    assert_eq!(msg2.get("Event"), Some("Second"));
}

#[test]
fn embedded_crlf_crlf_in_header_value() {
    // codec splits on the FIRST \r\n\r\n regardless of position — no escaping in AMI
    let mut codec = AmiCodec::new();
    let mut buf = BytesMut::from(
        "Asterisk Call Manager/6.0.0\r\n\
         Event: Test\r\n\
         Data: part1\r\n\
         \r\n\
         Leftover: part2\r\n\
         \r\n",
    );

    let msg1 = codec
        .decode(&mut buf)
        .expect("first decode should succeed")
        .expect("should produce first message");
    assert_eq!(msg1.get("Event"), Some("Test"));
    assert_eq!(msg1.get("Data"), Some("part1"));
    // second \r\n\r\n produces a separate message
    let msg2 = codec
        .decode(&mut buf)
        .expect("second decode should succeed")
        .expect("should produce second message");
    assert_eq!(msg2.get("Leftover"), Some("part2"));
}

#[test]
fn null_bytes_in_header_value() {
    // null bytes are valid utf-8 — from_utf8_lossy keeps them; just verify no panic
    let mut codec = AmiCodec::new();
    let banner = b"Asterisk Call Manager/6.0.0\r\n";
    let body = b"Event: Test\r\nKey: val\x00ue\r\n\r\n";
    let mut buf = BytesMut::with_capacity(banner.len() + body.len());
    buf.extend_from_slice(banner);
    buf.extend_from_slice(body);
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Event"), Some("Test"));
    assert!(msg.get("Key").is_some());
}

#[test]
fn non_utf8_bytes_in_header() {
    // 0xFF 0xFE are invalid utf-8; from_utf8_lossy replaces with U+FFFD
    let mut codec = AmiCodec::new();
    let banner = b"Asterisk Call Manager/6.0.0\r\n";
    let mut raw = Vec::new();
    raw.extend_from_slice(banner);
    raw.extend_from_slice(b"Event: Test\r\nKey: ");
    raw.extend_from_slice(&[0xFF, 0xFE]);
    raw.extend_from_slice(b"\r\n\r\n");
    let mut buf = BytesMut::from(&raw[..]);
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Event"), Some("Test"));
    // value should exist with replacement characters, not panic
    assert!(msg.get("Key").is_some());
}

#[test]
fn no_banner_rejects() {
    let mut codec = AmiCodec::new();
    let mut buf = BytesMut::from("NOT AN AMI SERVER\r\n");
    let err = codec
        .decode(&mut buf)
        .expect_err("should reject non-AMI banner");
    let msg = err.to_string();
    assert!(
        msg.contains("expected AMI banner"),
        "error should mention banner: {msg}"
    );
}

#[test]
fn wrong_banner_prefix_rejects() {
    let mut codec = AmiCodec::new();
    let mut buf = BytesMut::from("SIP/2.0 200 OK\r\n");
    let err = codec
        .decode(&mut buf)
        .expect_err("should reject SIP banner");
    let msg = err.to_string();
    assert!(
        msg.contains("expected AMI banner"),
        "error should mention banner: {msg}"
    );
}

#[test]
fn banner_partial_delivery() {
    let mut codec = AmiCodec::new();
    // partial banner — no \r\n yet
    let mut buf = BytesMut::from("Asterisk Call Man");
    let result = codec.decode(&mut buf).expect("partial should not error");
    assert!(result.is_none(), "partial banner should yield None");

    // complete the banner and add a message
    buf.extend_from_slice(b"ager/6.0.0\r\nEvent: Test\r\n\r\n");
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Event"), Some("Test"));
}

#[test]
fn chan_variable_nested_parens() {
    // ChanVariable(foo(bar)) — ends_with(')') matches outer paren
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Event: Test\r\n\
         ChanVariable(foo(bar)): value\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get_variable("foo(bar)"), Some("value"));
}

#[test]
fn chan_variable_empty_name() {
    // ChanVariable(): value — empty parens yields empty string key
    let mut codec = AmiCodec::new();
    let raw = with_banner(
        "Event: Test\r\n\
         ChanVariable(): value\r\n\
         \r\n",
    );
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    // empty string key extracted into channel_variables
    assert_eq!(msg.get_variable(""), Some("value"));
}

#[test]
fn header_with_no_value_after_colon() {
    let mut codec = AmiCodec::new();
    let raw = with_banner("Event: Test\r\nKey:\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Key"), Some(""));
}

#[test]
fn header_with_multiple_colons() {
    // only splits on the first colon
    let mut codec = AmiCodec::new();
    let raw = with_banner("Event: Test\r\nKey: val:ue:extra\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("Key"), Some("val:ue:extra"));
}

#[test]
fn message_exactly_at_size_limit() {
    // size check is > MAX_MESSAGE_SIZE, so exactly at limit should pass
    let mut codec = AmiCodec::new();
    let banner = b"Asterisk Call Manager/6.0.0\r\n";
    let max: usize = 64 * 1024;
    // feed banner + message as one buffer; total must be exactly max
    let overhead = banner.len() + b"K: \r\n\r\n".len(); // banner + minimal header frame
    let value_len = max - overhead;
    let value: String = "V".repeat(value_len);
    let body = format!("K: {value}\r\n\r\n");
    let mut buf = BytesMut::with_capacity(max);
    buf.extend_from_slice(banner);
    buf.extend_from_slice(body.as_bytes());
    assert_eq!(buf.len(), max, "buffer should be exactly at the limit");
    let msg = codec
        .decode(&mut buf)
        .expect("exactly at limit should succeed")
        .expect("should produce a message");
    assert_eq!(msg.get("K").expect("K header present").len(), value_len);
}

#[test]
fn empty_message_skipped() {
    // empty body (\r\n\r\n) has no headers -> codec skips it and returns next message
    let mut codec = AmiCodec::new();
    let raw = with_banner("\r\nEvent: Real\r\n\r\n");
    let mut buf = BytesMut::from(raw.as_str());
    let msg = codec
        .decode(&mut buf)
        .expect("decode should succeed")
        .expect("should skip empty and return real message");
    assert_eq!(msg.get("Event"), Some("Real"));
}
