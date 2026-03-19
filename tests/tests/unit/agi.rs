#![allow(clippy::unwrap_used)]

use asterisk_rs_agi::command;
use asterisk_rs_agi::error::AgiError;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::response::AgiResponse;
use asterisk_rs_agi::AgiChannel;
use asterisk_rs_core::error::ProtocolError;
use std::io::Cursor;
use tokio::io::{AsyncBufReadExt, AsyncWriteExt, BufReader};
use tokio::net::tcp::{OwnedReadHalf, OwnedWriteHalf};
use tokio::net::{TcpListener, TcpStream};

// ---------------------------------------------------------------------------
// request tests
// ---------------------------------------------------------------------------

async fn parse_request(input: &str) -> AgiRequest {
    let mut reader = BufReader::new(Cursor::new(input.as_bytes().to_vec()));
    AgiRequest::parse_from_reader(&mut reader)
        .await
        .expect("parse should succeed")
}

#[tokio::test]
async fn request_parse_standard_agi_request() {
    let input = "agi_network: yes\n\
        agi_network_script: test.agi\n\
        agi_request: agi://127.0.0.1/test\n\
        agi_channel: SIP/200-00000001\n\
        agi_language: en\n\
        agi_type: SIP\n\
        agi_uniqueid: 1234567890.42\n\
        agi_callerid: 2125551234\n\
        agi_calleridname: John Doe\n\
        agi_context: default\n\
        agi_extension: 100\n\
        agi_priority: 1\n\
        \n";
    let req = parse_request(input).await;
    assert_eq!(req.network(), Some("yes"));
    assert_eq!(req.network_script(), Some("test.agi"));
    assert_eq!(req.request(), Some("agi://127.0.0.1/test"));
    assert_eq!(req.channel(), Some("SIP/200-00000001"));
    assert_eq!(req.language(), Some("en"));
    assert_eq!(req.channel_type(), Some("SIP"));
    assert_eq!(req.unique_id(), Some("1234567890.42"));
    assert_eq!(req.caller_id(), Some("2125551234"));
    assert_eq!(req.caller_id_name(), Some("John Doe"));
    assert_eq!(req.context(), Some("default"));
    assert_eq!(req.extension(), Some("100"));
    assert_eq!(req.priority(), Some("1"));
}

#[tokio::test]
async fn request_parse_strips_agi_prefix() {
    let req = parse_request("agi_language: en\n\n").await;
    // stored without prefix
    assert_eq!(req.get("language"), Some("en"));
    // original key with prefix is not stored
    assert_eq!(req.get("agi_language"), None);
}

#[tokio::test]
async fn request_parse_preserves_non_agi_keys() {
    let req = parse_request("custom_var: hello\n\n").await;
    assert_eq!(req.get("custom_var"), Some("hello"));
}

#[tokio::test]
async fn request_parse_empty_input() {
    let req = parse_request("").await;
    assert_eq!(req.network(), None);
    assert_eq!(req.channel(), None);
}

#[tokio::test]
async fn request_parse_eof_without_blank_line() {
    // no trailing blank line — parser reads until eof
    let req = parse_request("agi_language: en\nagi_type: SIP").await;
    assert_eq!(req.language(), Some("en"));
    assert_eq!(req.channel_type(), Some("SIP"));
}

#[tokio::test]
async fn request_parse_ignores_lines_without_colon() {
    let input = "agi_language: en\ngarbage line\nagi_type: SIP\n\n";
    let req = parse_request(input).await;
    assert_eq!(req.language(), Some("en"));
    assert_eq!(req.channel_type(), Some("SIP"));
}

#[tokio::test]
async fn request_parse_value_with_colons() {
    // value contains colons — only split on first
    let req = parse_request("agi_request: agi://host:4573/script\n\n").await;
    assert_eq!(req.request(), Some("agi://host:4573/script"));
}

#[tokio::test]
async fn request_parse_whitespace_trimming() {
    let req = parse_request("  agi_language  :  en  \n\n").await;
    assert_eq!(req.language(), Some("en"));
}

#[tokio::test]
async fn request_get_returns_none_for_missing_key() {
    let req = parse_request("agi_language: en\n\n").await;
    assert_eq!(req.get("nonexistent"), None);
}

#[tokio::test]
async fn request_get_arbitrary_variable() {
    let req = parse_request("custom_var: world\n\n").await;
    assert_eq!(req.get("custom_var"), Some("world"));
}

#[tokio::test]
async fn request_all_accessors_return_none_on_empty() {
    let req = parse_request("\n").await;
    assert_eq!(req.network(), None);
    assert_eq!(req.network_script(), None);
    assert_eq!(req.request(), None);
    assert_eq!(req.channel(), None);
    assert_eq!(req.language(), None);
    assert_eq!(req.channel_type(), None);
    assert_eq!(req.unique_id(), None);
    assert_eq!(req.caller_id(), None);
    assert_eq!(req.caller_id_name(), None);
    assert_eq!(req.context(), None);
    assert_eq!(req.extension(), None);
    assert_eq!(req.priority(), None);
}

// ---------------------------------------------------------------------------
// response tests
// ---------------------------------------------------------------------------

#[test]
fn response_parse_success_simple() {
    let resp = AgiResponse::parse("200 result=1").expect("should parse");
    assert_eq!(resp.code, 200);
    assert_eq!(resp.result, 1);
    assert!(resp.data.is_none());
    assert!(resp.endpos.is_none());
}

#[test]
fn response_parse_success_with_data() {
    let resp = AgiResponse::parse("200 result=0 (timeout)").expect("should parse");
    assert_eq!(resp.code, 200);
    assert_eq!(resp.result, 0);
    assert_eq!(resp.data.as_deref(), Some("timeout"));
    assert!(resp.endpos.is_none());
}

#[test]
fn response_parse_success_with_endpos() {
    let resp = AgiResponse::parse("200 result=0 endpos=12345").expect("should parse");
    assert_eq!(resp.code, 200);
    assert_eq!(resp.result, 0);
    assert!(resp.data.is_none());
    assert_eq!(resp.endpos, Some(12345));
}

#[test]
fn response_parse_success_with_data_and_endpos() {
    let resp = AgiResponse::parse("200 result=1 (dtmf) endpos=67890").expect("should parse");
    assert_eq!(resp.code, 200);
    assert_eq!(resp.result, 1);
    assert_eq!(resp.data.as_deref(), Some("dtmf"));
    assert_eq!(resp.endpos, Some(67890));
}

#[test]
fn response_parse_invalid_command() {
    let resp = AgiResponse::parse("510 Invalid or unknown command").expect("should parse");
    assert_eq!(resp.code, 510);
    assert_eq!(resp.result, -1);
}

#[test]
fn response_parse_dead_channel() {
    let resp =
        AgiResponse::parse("511 Command Not Permitted on a dead channel or intercept routine")
            .expect("should parse");
    assert_eq!(resp.code, 511);
    assert_eq!(resp.result, -1);
}

#[test]
fn response_parse_negative_result() {
    let resp = AgiResponse::parse("200 result=-1").expect("should parse");
    assert_eq!(resp.code, 200);
    assert_eq!(resp.result, -1);
}

// ---------------------------------------------------------------------------
// command tests
// ---------------------------------------------------------------------------

#[test]
fn command_format_simple_command() {
    assert_eq!(command::format_command(command::ANSWER, &[]), "ANSWER\n");
}

#[test]
fn command_format_command_with_args() {
    assert_eq!(
        command::format_command(command::STREAM_FILE, &["hello-world", "#"]),
        "STREAM FILE hello-world #\n"
    );
}

#[test]
fn command_format_command_with_spaces_in_arg() {
    assert_eq!(
        command::format_command(command::VERBOSE, &["hello world", "1"]),
        "VERBOSE \"hello world\" 1\n"
    );
}

#[test]
fn command_format_command_with_embedded_quotes() {
    assert_eq!(
        command::format_command(command::VERBOSE, &["say \"hi\"", "1"]),
        "VERBOSE \"say \\\"hi\\\"\" 1\n"
    );
}

#[test]
fn command_format_hangup_with_optional_channel() {
    assert_eq!(
        command::format_command(command::HANGUP, &["SIP/100-00000001"]),
        "HANGUP SIP/100-00000001\n"
    );
}

#[test]
fn command_format_record_file_command() {
    let cmd = command::format_command(command::RECORD_FILE, &["myfile", "wav", "#", "5000"]);
    assert_eq!(cmd, "RECORD FILE myfile wav # 5000\n");
}

#[test]
fn command_format_database_get_command() {
    let cmd = command::format_command(command::DATABASE_GET, &["cidname", "12125551234"]);
    assert_eq!(cmd, "DATABASE GET cidname 12125551234\n");
}

#[test]
fn command_format_gosub_command() {
    let cmd = command::format_command(command::GOSUB, &["default", "s", "1"]);
    assert_eq!(cmd, "GOSUB default s 1\n");
}

#[test]
fn command_format_say_alpha_command() {
    let cmd = command::format_command(command::SAY_ALPHA, &["hello", "#"]);
    assert_eq!(cmd, "SAY ALPHA hello #\n");
}

#[test]
fn command_format_speech_create_command() {
    let cmd = command::format_command(command::SPEECH_CREATE, &["lumenvox"]);
    assert_eq!(cmd, "SPEECH CREATE lumenvox\n");
}

#[test]
fn command_format_set_callerid_command() {
    let cmd = command::format_command(command::SET_CALLERID, &["\"John\" <1234>"]);
    assert_eq!(cmd, "SET CALLERID \"\\\"John\\\" <1234>\"\n");
}

#[test]
fn command_format_set_music_command() {
    let cmd = command::format_command(command::SET_MUSIC, &["on", "default"]);
    assert_eq!(cmd, "SET MUSIC on default\n");
}

// ---------------------------------------------------------------------------
// error tests
// ---------------------------------------------------------------------------

#[test]
fn error_io_error_display() {
    let io_err = std::io::Error::new(std::io::ErrorKind::BrokenPipe, "pipe broke");
    let err = AgiError::Io(io_err);
    let msg = err.to_string();
    assert!(
        msg.contains("pipe broke"),
        "expected io error details in: {msg}"
    );
}

#[test]
fn error_channel_hung_up_display() {
    let err = AgiError::ChannelHungUp;
    assert_eq!(err.to_string(), "channel hung up during AGI session");
}

#[test]
fn error_invalid_response_display() {
    let err = AgiError::InvalidResponse {
        raw: "garbage".to_owned(),
    };
    let msg = err.to_string();
    assert!(msg.contains("garbage"), "expected raw string in: {msg}");
}

#[test]
fn error_command_failed_display() {
    let err = AgiError::CommandFailed {
        code: 510,
        message: "invalid command".to_owned(),
    };
    let msg = err.to_string();
    assert!(msg.contains("510"), "expected code in: {msg}");
    assert!(
        msg.contains("invalid command"),
        "expected message in: {msg}"
    );
}

#[test]
fn error_protocol_error_display() {
    let proto = ProtocolError::MalformedMessage {
        details: "bad frame".to_owned(),
    };
    let err = AgiError::Protocol(proto);
    let msg = err.to_string();
    assert!(
        msg.contains("bad frame"),
        "expected protocol details in: {msg}"
    );
}

#[test]
fn error_errors_are_send_sync() {
    fn assert_send_sync<T: Send + Sync>() {}
    assert_send_sync::<AgiError>();
}

#[test]
fn error_io_error_from_conversion() {
    let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "missing");
    let err: AgiError = io_err.into();
    assert!(matches!(err, AgiError::Io(_)));
}

#[test]
fn error_protocol_error_from_conversion() {
    let proto = ProtocolError::UnsupportedVersion {
        version: "99".to_owned(),
    };
    let err: AgiError = proto.into();
    assert!(matches!(err, AgiError::Protocol(_)));
}

// ---------------------------------------------------------------------------
// channel tests
// ---------------------------------------------------------------------------

/// create a connected channel pair with server-side reader/writer
async fn mock_channel() -> (AgiChannel, BufReader<OwnedReadHalf>, OwnedWriteHalf) {
    let listener = TcpListener::bind("127.0.0.1:0").await.expect("bind");
    let addr = listener.local_addr().expect("addr");
    let client = TcpStream::connect(addr).await.expect("connect");
    let (server, _) = listener.accept().await.expect("accept");
    let (client_read, client_write) = client.into_split();
    let (server_read, server_write) = server.into_split();
    let channel = AgiChannel::new(BufReader::new(client_read), client_write);
    (channel, BufReader::new(server_read), server_write)
}

/// spawn server handler that reads one command and responds with 200 result=0,
/// returns the command string the client sent
async fn run_ok(mut sr: BufReader<OwnedReadHalf>, mut sw: OwnedWriteHalf) -> String {
    let handle = tokio::spawn(async move {
        let mut cmd = String::new();
        sr.read_line(&mut cmd).await.expect("read command");
        sw.write_all(b"200 result=0\n")
            .await
            .expect("write response");
        sw.flush().await.expect("flush");
        cmd
    });
    handle.await.expect("server task")
}

#[tokio::test]
async fn channel_answer() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    let resp = ch.answer().await.expect("answer");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "ANSWER\n");
    assert_eq!(resp.code, 200);
}

#[tokio::test]
async fn channel_hangup_no_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.hangup(None).await.expect("hangup");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "HANGUP\n");
}

#[tokio::test]
async fn channel_hangup_with_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.hangup(Some("SIP/100")).await.expect("hangup");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "HANGUP SIP/100\n");
}

#[tokio::test]
async fn channel_stream_file_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.stream_file("hello-world", "0123456789")
        .await
        .expect("stream_file");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "STREAM FILE hello-world 0123456789\n");
}

#[tokio::test]
async fn channel_get_data_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_data("welcome", 5000, 4).await.expect("get_data");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET DATA welcome 5000 4\n");
}

#[tokio::test]
async fn channel_say_digits_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_digits("12345", "#").await.expect("say_digits");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY DIGITS 12345 #\n");
}

#[tokio::test]
async fn channel_say_number_positive() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_number(42, "#").await.expect("say_number");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY NUMBER 42 #\n");
}

#[tokio::test]
async fn channel_say_number_negative() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_number(-7, "#").await.expect("say_number negative");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY NUMBER -7 #\n");
}

#[tokio::test]
async fn channel_set_variable_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_variable("MY_VAR", "my_value")
        .await
        .expect("set_variable");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET VARIABLE MY_VAR my_value\n");
}

#[tokio::test]
async fn channel_get_variable_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_variable("CALLERID").await.expect("get_variable");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET VARIABLE CALLERID\n");
}

#[tokio::test]
async fn channel_exec_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.exec("Playback", "hello-world").await.expect("exec");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "EXEC Playback hello-world\n");
}

#[tokio::test]
async fn channel_wait_for_digit_with_timeout() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.wait_for_digit(5000).await.expect("wait_for_digit");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "WAIT FOR DIGIT 5000\n");
}

#[tokio::test]
async fn channel_wait_for_digit_infinite() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.wait_for_digit(-1)
        .await
        .expect("wait_for_digit infinite");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "WAIT FOR DIGIT -1\n");
}

#[tokio::test]
async fn channel_channel_status_no_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.channel_status(None).await.expect("channel_status");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "CHANNEL STATUS\n");
}

#[tokio::test]
async fn channel_channel_status_with_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.channel_status(Some("SIP/100"))
        .await
        .expect("channel_status");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "CHANNEL STATUS SIP/100\n");
}

#[tokio::test]
async fn channel_verbose_sends_quoted_message() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.verbose("test message", 3).await.expect("verbose");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "VERBOSE \"test message\" 3\n");
}

#[tokio::test]
async fn channel_set_callerid_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_callerid("5551234567").await.expect("set_callerid");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET CALLERID 5551234567\n");
}

#[tokio::test]
async fn channel_database_get_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.database_get("cidname", "5551234567")
        .await
        .expect("database_get");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "DATABASE GET cidname 5551234567\n");
}

#[tokio::test]
async fn channel_database_put_with_quoted_value() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.database_put("cidname", "5551234567", "John Doe")
        .await
        .expect("database_put");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "DATABASE PUT cidname 5551234567 \"John Doe\"\n");
}

#[tokio::test]
async fn channel_database_del_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.database_del("cidname", "5551234567")
        .await
        .expect("database_del");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "DATABASE DEL cidname 5551234567\n");
}

#[tokio::test]
async fn channel_database_deltree_with_key() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.database_deltree("cidname", Some("mykey"))
        .await
        .expect("database_deltree");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "DATABASE DELTREE cidname mykey\n");
}

#[tokio::test]
async fn channel_database_deltree_without_key() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.database_deltree("cidname", None)
        .await
        .expect("database_deltree");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "DATABASE DELTREE cidname\n");
}

#[tokio::test]
async fn channel_control_stream_file_all_params() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.control_stream_file(
        "hello-world",
        "#",
        Some(5000),
        Some("*"),
        Some("0"),
        Some("9"),
    )
    .await
    .expect("control_stream_file");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "CONTROL STREAM FILE hello-world # 5000 * 0 9\n");
}

#[tokio::test]
async fn channel_control_stream_file_defaults() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.control_stream_file("hello-world", "#", None, None, None, None)
        .await
        .expect("control_stream_file defaults");
    let cmd = server.await.expect("server");
    // default skipms 3000, empty strings for optional chars
    assert_eq!(cmd, "CONTROL STREAM FILE hello-world # 3000   \n");
}

#[tokio::test]
async fn channel_get_full_variable_with_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_full_variable("${CALLERID(num)}", Some("SIP/100"))
        .await
        .expect("get_full_variable");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET FULL VARIABLE ${CALLERID(num)} SIP/100\n");
}

#[tokio::test]
async fn channel_get_full_variable_without_channel() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_full_variable("${EXTEN}", None)
        .await
        .expect("get_full_variable");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET FULL VARIABLE ${EXTEN}\n");
}

#[tokio::test]
async fn channel_get_option_with_timeout() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_option("welcome", "#", Some(5000))
        .await
        .expect("get_option");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET OPTION welcome # 5000\n");
}

#[tokio::test]
async fn channel_get_option_without_timeout() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.get_option("welcome", "#", None)
        .await
        .expect("get_option");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GET OPTION welcome #\n");
}

#[tokio::test]
async fn channel_gosub_with_args() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.gosub("default", "s", "1", Some("arg1,arg2"))
        .await
        .expect("gosub");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GOSUB default s 1 arg1,arg2\n");
}

#[tokio::test]
async fn channel_gosub_without_args() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.gosub("default", "s", "1", None).await.expect("gosub");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "GOSUB default s 1\n");
}

#[tokio::test]
async fn channel_noop_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.noop().await.expect("noop");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "NOOP\n");
}

#[tokio::test]
async fn channel_receive_char_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.receive_char(2000).await.expect("receive_char");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "RECEIVE CHAR 2000\n");
}

#[tokio::test]
async fn channel_receive_text_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.receive_text(3000).await.expect("receive_text");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "RECEIVE TEXT 3000\n");
}

#[tokio::test]
async fn channel_record_file_with_beep_and_silence() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.record_file("recording", "wav", "#", 30000, true, Some(5))
        .await
        .expect("record_file");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "RECORD FILE recording wav # 30000 beep s=5\n");
}

#[tokio::test]
async fn channel_record_file_without_beep_and_silence() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.record_file("recording", "wav", "#", 30000, false, None)
        .await
        .expect("record_file");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "RECORD FILE recording wav # 30000\n");
}

#[tokio::test]
async fn channel_say_alpha_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_alpha("hello", "#").await.expect("say_alpha");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY ALPHA hello #\n");
}

#[tokio::test]
async fn channel_say_date_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_date(1_234_567_890, "#").await.expect("say_date");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY DATE 1234567890 #\n");
}

#[tokio::test]
async fn channel_say_datetime_with_format_and_timezone() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_datetime(1_234_567_890, "#", Some("IMp"), Some("EST"))
        .await
        .expect("say_datetime");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY DATETIME 1234567890 # IMp EST\n");
}

#[tokio::test]
async fn channel_say_datetime_without_format() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_datetime(1_234_567_890, "#", None, None)
        .await
        .expect("say_datetime");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY DATETIME 1234567890 #\n");
}

#[tokio::test]
async fn channel_say_phonetic_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_phonetic("hello", "#").await.expect("say_phonetic");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY PHONETIC hello #\n");
}

#[tokio::test]
async fn channel_say_time_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.say_time(1_234_567_890, "#").await.expect("say_time");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SAY TIME 1234567890 #\n");
}

#[tokio::test]
async fn channel_send_image_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.send_image("myimage").await.expect("send_image");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SEND IMAGE myimage\n");
}

#[tokio::test]
async fn channel_send_text_quotes_spaces() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.send_text("hello world").await.expect("send_text");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SEND TEXT \"hello world\"\n");
}

#[tokio::test]
async fn channel_set_autohangup_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_autohangup(30).await.expect("set_autohangup");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET AUTOHANGUP 30\n");
}

#[tokio::test]
async fn channel_set_context_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_context("default").await.expect("set_context");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET CONTEXT default\n");
}

#[tokio::test]
async fn channel_set_extension_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_extension("100").await.expect("set_extension");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET EXTENSION 100\n");
}

#[tokio::test]
async fn channel_set_music_enabled_with_class() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_music(true, Some("default"))
        .await
        .expect("set_music on");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET MUSIC on default\n");
}

#[tokio::test]
async fn channel_set_music_disabled_without_class() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_music(false, None).await.expect("set_music off");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET MUSIC off\n");
}

#[tokio::test]
async fn channel_set_priority_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.set_priority("1").await.expect("set_priority");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SET PRIORITY 1\n");
}

#[tokio::test]
async fn channel_speech_create_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_create("flite").await.expect("speech_create");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH CREATE flite\n");
}

#[tokio::test]
async fn channel_speech_destroy_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_destroy().await.expect("speech_destroy");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH DESTROY\n");
}

#[tokio::test]
async fn channel_speech_activate_grammar_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_activate_grammar("mygrammar")
        .await
        .expect("speech_activate_grammar");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH ACTIVATE GRAMMAR mygrammar\n");
}

#[tokio::test]
async fn channel_speech_deactivate_grammar_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_deactivate_grammar("mygrammar")
        .await
        .expect("speech_deactivate_grammar");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH DEACTIVATE GRAMMAR mygrammar\n");
}

#[tokio::test]
async fn channel_speech_load_grammar_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_load_grammar("mygrammar", "/path/to/grammar")
        .await
        .expect("speech_load_grammar");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH LOAD GRAMMAR mygrammar /path/to/grammar\n");
}

#[tokio::test]
async fn channel_speech_unload_grammar_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_unload_grammar("mygrammar")
        .await
        .expect("speech_unload_grammar");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH UNLOAD GRAMMAR mygrammar\n");
}

#[tokio::test]
async fn channel_speech_recognize_with_offset() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_recognize("beep", 5000, Some(1000))
        .await
        .expect("speech_recognize");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH RECOGNIZE beep 5000 1000\n");
}

#[tokio::test]
async fn channel_speech_recognize_without_offset() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_recognize("beep", 5000, None)
        .await
        .expect("speech_recognize");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH RECOGNIZE beep 5000\n");
}

#[tokio::test]
async fn channel_speech_set_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.speech_set("name", "value").await.expect("speech_set");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "SPEECH SET name value\n");
}

#[tokio::test]
async fn channel_tdd_mode_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.tdd_mode("on").await.expect("tdd_mode");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "TDD MODE on\n");
}

#[tokio::test]
async fn channel_asyncagi_break_sends_correct_command() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(run_ok(sr, sw));
    ch.asyncagi_break().await.expect("asyncagi_break");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "ASYNCAGI BREAK\n");
}

// -- send_command edge cases --

#[tokio::test]
async fn channel_send_command_when_hung_up_returns_error() {
    let (mut ch, sr, sw) = mock_channel().await;
    // respond with 511 to set hung_up flag
    let server = tokio::spawn(async move {
        let mut sr = sr;
        let mut sw = sw;
        let mut cmd = String::new();
        sr.read_line(&mut cmd).await.expect("read");
        sw.write_all(b"511 result=-1\n").await.expect("write 511");
        sw.flush().await.expect("flush");
    });
    let err = ch.answer().await.expect_err("expected hung up");
    server.await.expect("server");
    assert!(
        matches!(err, AgiError::ChannelHungUp),
        "expected ChannelHungUp from 511, got {err:?}"
    );
    // subsequent call should fail immediately without network io
    let err = ch.answer().await.expect_err("expected hung up on retry");
    assert!(
        matches!(err, AgiError::ChannelHungUp),
        "expected ChannelHungUp on retry, got {err:?}"
    );
}

#[tokio::test]
async fn channel_send_command_on_eof_sets_hung_up() {
    let (mut ch, sr, sw) = mock_channel().await;
    // server reads command then drops connection
    let server = tokio::spawn(async move {
        let mut sr = sr;
        let mut cmd = String::new();
        sr.read_line(&mut cmd).await.expect("read");
        drop(sw);
    });
    let err = ch.answer().await.expect_err("expected hung up on eof");
    server.await.expect("server");
    assert!(
        matches!(err, AgiError::ChannelHungUp),
        "expected ChannelHungUp on eof, got {err:?}"
    );
}

#[tokio::test]
async fn channel_send_command_on_511_response_sets_hung_up() {
    let (mut ch, sr, sw) = mock_channel().await;
    let server = tokio::spawn(async move {
        let mut sr = sr;
        let mut sw = sw;
        let mut cmd = String::new();
        sr.read_line(&mut cmd).await.expect("read");
        sw.write_all(b"511 result=-1\n").await.expect("write 511");
        sw.flush().await.expect("flush");
        cmd
    });
    let err = ch.answer().await.expect_err("expected 511 error");
    let cmd = server.await.expect("server");
    assert_eq!(cmd, "ANSWER\n");
    assert!(
        matches!(err, AgiError::ChannelHungUp),
        "expected ChannelHungUp, got {err:?}"
    );
}
