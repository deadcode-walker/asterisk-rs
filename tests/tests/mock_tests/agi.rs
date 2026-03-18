use std::time::Duration;

use asterisk_rs_agi::channel::AgiChannel;
use asterisk_rs_agi::error::AgiError;
use asterisk_rs_agi::handler::AgiHandler;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::server::AgiServer;
use asterisk_rs_tests::helpers::init_tracing;
use asterisk_rs_tests::mock::agi_client::{free_port, standard_env, MockAgiClient};
use tokio::sync::mpsc;

// ---------------------------------------------------------------------------
// handlers
// ---------------------------------------------------------------------------

/// handler that answers then hangs up
struct AnswerAndHangup;

impl AgiHandler for AnswerAndHangup {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        channel.answer().await?;
        channel.hangup(None).await?;
        Ok(())
    }
}

/// captures request metadata and forwards it through a channel
struct CapturedSession {
    channel_name: Option<String>,
    unique_id: Option<String>,
    caller_id: Option<String>,
    context: Option<String>,
}

struct CapturingHandler {
    tx: mpsc::Sender<CapturedSession>,
}

impl AgiHandler for CapturingHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let _ = self
            .tx
            .send(CapturedSession {
                channel_name: request.channel().map(String::from),
                unique_id: request.unique_id().map(String::from),
                caller_id: request.caller_id().map(String::from),
                context: request.context().map(String::from),
            })
            .await;
        // send a command so the mock client has something to respond to
        channel.answer().await?;
        Ok(())
    }
}

/// handler that tries answer then stream_file, exposing the hangup error
struct HangupDetector {
    tx: mpsc::Sender<String>,
}

impl AgiHandler for HangupDetector {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        channel.answer().await?;
        match channel.stream_file("hello", "").await {
            Err(AgiError::ChannelHungUp) => {
                let _ = self.tx.send("hungup".into()).await;
            }
            other => {
                let _ = self.tx.send(format!("unexpected: {other:?}")).await;
            }
        }
        Ok(())
    }
}

/// handler that blocks until notified, useful for concurrency tests
struct BlockingHandler {
    ready_tx: mpsc::Sender<()>,
    gate_rx: tokio::sync::watch::Receiver<bool>,
}

impl AgiHandler for BlockingHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        // signal that we entered the handler
        let _ = self.ready_tx.send(()).await;
        // wait until the test sets the gate to true
        let mut rx = self.gate_rx.clone();
        while !*rx.borrow_and_update() {
            rx.changed().await.expect("gate watch closed");
        }
        // drain a command so the mock client can finish
        let _ = channel.answer().await;
        Ok(())
    }
}

// ---------------------------------------------------------------------------
// helper
// ---------------------------------------------------------------------------

/// build a server on a free port, return (join handle, shutdown, addr)
async fn spawn_server<H: AgiHandler>(
    handler: H,
    max_connections: Option<usize>,
) -> (
    tokio::task::JoinHandle<asterisk_rs_agi::error::Result<()>>,
    asterisk_rs_agi::ShutdownHandle,
    std::net::SocketAddr,
) {
    let port = free_port().await;
    let addr: std::net::SocketAddr = format!("127.0.0.1:{port}")
        .parse()
        .expect("valid socket addr");

    let mut builder = AgiServer::builder()
        .bind(format!("127.0.0.1:{port}"))
        .handler(handler);

    if let Some(n) = max_connections {
        builder = builder.max_connections(n);
    }

    let (server, shutdown) = builder.build().await.expect("failed to build AGI server");
    let handle = tokio::spawn(server.run());

    // small yield so the listener is ready
    tokio::time::sleep(Duration::from_millis(20)).await;

    (handle, shutdown, addr)
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn server_accepts_connection() {
    init_tracing();

    let (handle, shutdown, addr) = spawn_server(AnswerAndHangup, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // handler sends ANSWER then HANGUP
    let cmd1 = client.read_command().await.expect("should read ANSWER");
    assert_eq!(cmd1, "ANSWER", "first command should be ANSWER");
    client.send_success(0).await;

    let cmd2 = client.read_command().await.expect("should read HANGUP");
    assert_eq!(cmd2, "HANGUP", "second command should be HANGUP");
    client.send_success(1).await;

    // connection should close after handler returns
    let eof = client.read_command().await;
    assert!(
        eof.is_none(),
        "stream should be closed after handler completes"
    );

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn handler_receives_request() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel::<CapturedSession>(1);
    let handler = CapturingHandler { tx };
    let (handle, shutdown, addr) = spawn_server(handler, None).await;

    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // handler sends capture then ANSWER, we need to respond to unblock it
    let session = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for captured session")
        .expect("channel closed unexpectedly");

    assert_eq!(session.channel_name.as_deref(), Some("SIP/100-00000001"));
    assert_eq!(session.unique_id.as_deref(), Some("1234567890.1"));
    assert_eq!(session.caller_id.as_deref(), Some("100"));
    assert_eq!(session.context.as_deref(), Some("default"));

    // respond to the ANSWER command so handler can complete
    let cmd = client.read_command().await.expect("should read ANSWER");
    assert_eq!(cmd, "ANSWER");
    client.send_success(0).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn handler_sends_commands() {
    init_tracing();

    let (handle, shutdown, addr) = spawn_server(AnswerAndHangup, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // verify AGI command format — commands should be uppercase with \n terminator
    let cmd1 = client
        .read_command()
        .await
        .expect("should read first command");
    assert_eq!(cmd1, "ANSWER", "ANSWER command format");
    client.send_success(0).await;

    let cmd2 = client
        .read_command()
        .await
        .expect("should read second command");
    assert_eq!(cmd2, "HANGUP", "HANGUP command format");
    client.send_success(1).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn channel_hungup_detection() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel::<String>(1);
    let handler = HangupDetector { tx };
    let (handle, shutdown, addr) = spawn_server(handler, None).await;

    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // respond to ANSWER normally
    let cmd = client.read_command().await.expect("should read ANSWER");
    assert_eq!(cmd, "ANSWER");
    client.send_success(0).await;

    // respond to STREAM FILE with 511 hungup
    let cmd = client
        .read_command()
        .await
        .expect("should read STREAM FILE");
    assert!(
        cmd.starts_with("STREAM FILE"),
        "expected STREAM FILE, got: {cmd}"
    );
    client.send_hangup().await;

    // handler should detect the hangup
    let signal = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for hangup signal")
        .expect("channel closed unexpectedly");
    assert_eq!(signal, "hungup", "handler should detect ChannelHungUp");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn shutdown_handle() {
    init_tracing();

    let (handle, shutdown, _addr) = spawn_server(AnswerAndHangup, None).await;

    // shutdown without any connections
    shutdown.shutdown();

    let result = tokio::time::timeout(Duration::from_secs(5), handle)
        .await
        .expect("server should stop within timeout")
        .expect("task should not panic");
    result.expect("server should exit with Ok(())");
}

#[tokio::test]
async fn max_connections_enforced() {
    init_tracing();

    // gate mechanism: handler blocks until watch is set to true
    let (ready_tx, mut ready_rx) = mpsc::channel::<()>(2);
    let (gate_tx, gate_rx) = tokio::sync::watch::channel(false);

    let handler = BlockingHandler { ready_tx, gate_rx };
    let (handle, shutdown, addr) = spawn_server(handler, Some(1)).await;

    let env = standard_env();

    // first connection — should enter handler and block
    let mut client1 = MockAgiClient::connect(addr, &env).await;
    tokio::time::timeout(Duration::from_secs(5), ready_rx.recv())
        .await
        .expect("timed out waiting for first handler entry")
        .expect("ready channel closed");

    // second connection — should connect at TCP level but handler won't be invoked
    // because the semaphore permit is held by client1
    let mut client2 = MockAgiClient::connect(addr, &env).await;

    // give a moment and confirm second handler hasn't entered
    tokio::time::sleep(Duration::from_millis(100)).await;
    assert!(
        ready_rx.try_recv().is_err(),
        "second handler should not have entered while first holds the permit"
    );

    // release first handler by setting gate to true
    gate_tx.send(true).expect("failed to release handlers");
    // respond to client1's ANSWER
    let cmd = client1
        .read_command()
        .await
        .expect("should read ANSWER from client1");
    assert_eq!(cmd, "ANSWER");
    client1.send_success(0).await;

    // now second handler should enter
    tokio::time::timeout(Duration::from_secs(5), ready_rx.recv())
        .await
        .expect("timed out waiting for second handler entry")
        .expect("ready channel closed");

    // second handler is already unblocked (gate is true), respond to ANSWER
    let cmd = client2
        .read_command()
        .await
        .expect("should read ANSWER from client2");
    assert_eq!(cmd, "ANSWER");
    client2.send_success(0).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn concurrent_sessions() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel::<CapturedSession>(3);
    let handler = CapturingHandler { tx };
    let (handle, shutdown, addr) = spawn_server(handler, None).await;

    let env = standard_env();

    // spawn 3 concurrent mock clients
    let mut handles = Vec::new();
    for _ in 0..3 {
        let env = env.clone();
        let task_addr = addr;
        handles.push(tokio::spawn(async move {
            let mut client = MockAgiClient::connect(task_addr, &env).await;
            // each handler sends ANSWER
            let cmd = client.read_command().await.expect("should read ANSWER");
            assert_eq!(cmd, "ANSWER");
            client.send_success(0).await;
        }));
    }

    // collect all 3 captured sessions
    for _ in 0..3 {
        let session = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for session capture")
            .expect("channel closed unexpectedly");
        assert_eq!(session.channel_name.as_deref(), Some("SIP/100-00000001"));
    }

    // wait for all mock clients to finish
    for h in handles {
        h.await.expect("mock client task should not panic");
    }

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn commands_with_arguments() {
    init_tracing();

    // handler that exercises several command types with arguments
    struct MultiCommandHandler;

    impl AgiHandler for MultiCommandHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.answer().await?;
            channel.stream_file("hello-world", "#").await?;
            channel.set_variable("MY_VAR", "some value").await?;
            channel.get_variable("CHANNEL").await?;
            channel.exec("Playback", "silence/1").await?;
            channel.say_digits("12345", "").await?;
            channel.verbose("test message", 3).await?;
            channel.noop().await?;
            channel.hangup(None).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(MultiCommandHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // ANSWER
    let cmd = client.read_command().await.expect("ANSWER");
    assert_eq!(cmd, "ANSWER");
    client.send_success(0).await;

    // STREAM FILE hello-world #
    let cmd = client.read_command().await.expect("STREAM FILE");
    assert!(cmd.starts_with("STREAM FILE"), "got: {cmd}");
    assert!(
        cmd.contains("hello-world"),
        "should contain filename: {cmd}"
    );
    assert!(cmd.contains("#"), "should contain escape digits: {cmd}");
    client.send_response("200 result=0 endpos=3000").await;

    // SET VARIABLE MY_VAR "some value" (quoted because of space)
    let cmd = client.read_command().await.expect("SET VARIABLE");
    assert!(cmd.starts_with("SET VARIABLE"), "got: {cmd}");
    assert!(cmd.contains("MY_VAR"), "should contain var name: {cmd}");
    assert!(cmd.contains("some value"), "should contain value: {cmd}");
    client.send_success(1).await;

    // GET VARIABLE CHANNEL
    let cmd = client.read_command().await.expect("GET VARIABLE");
    assert!(cmd.starts_with("GET VARIABLE"), "got: {cmd}");
    assert!(cmd.contains("CHANNEL"), "should contain var name: {cmd}");
    client.send_response("200 result=1 (SIP/100-0001)").await;

    // EXEC Playback silence/1
    let cmd = client.read_command().await.expect("EXEC");
    assert!(cmd.starts_with("EXEC"), "got: {cmd}");
    assert!(cmd.contains("Playback"), "should contain app name: {cmd}");
    assert!(cmd.contains("silence/1"), "should contain app args: {cmd}");
    client.send_success(0).await;

    // SAY DIGITS 12345 ""
    let cmd = client.read_command().await.expect("SAY DIGITS");
    assert!(cmd.starts_with("SAY DIGITS"), "got: {cmd}");
    assert!(cmd.contains("12345"), "should contain digits: {cmd}");
    client.send_success(0).await;

    // VERBOSE "test message" 3 (quoted because of space)
    let cmd = client.read_command().await.expect("VERBOSE");
    assert!(cmd.starts_with("VERBOSE"), "got: {cmd}");
    assert!(
        cmd.contains("test message"),
        "should contain message: {cmd}"
    );
    assert!(cmd.contains("3"), "should contain level: {cmd}");
    client.send_success(1).await;

    // NOOP
    let cmd = client.read_command().await.expect("NOOP");
    assert_eq!(cmd, "NOOP");
    client.send_success(0).await;

    // HANGUP
    let cmd = client.read_command().await.expect("HANGUP");
    assert_eq!(cmd, "HANGUP");
    client.send_success(1).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn all_say_commands() {
    init_tracing();

    struct SayHandler;

    impl AgiHandler for SayHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.say_number(42, "").await?;
            channel.say_alpha("hello", "").await?;
            channel.say_date(1700000000, "").await?;
            channel.say_datetime(1700000000, "", None, None).await?;
            channel.say_phonetic("world", "").await?;
            channel.say_time(1700000000, "").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SayHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let expected_prefixes = [
        "SAY NUMBER",
        "SAY ALPHA",
        "SAY DATE",
        "SAY DATETIME",
        "SAY PHONETIC",
        "SAY TIME",
    ];

    for prefix in &expected_prefixes {
        let cmd = client
            .read_command()
            .await
            .unwrap_or_else(|| panic!("should read {prefix} command"));
        assert!(
            cmd.starts_with(prefix),
            "expected command starting with {prefix}, got: {cmd}"
        );
        client.send_success(0).await;
    }

    // handler done, connection closes
    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn database_operations() {
    init_tracing();

    struct DbHandler;

    impl AgiHandler for DbHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.database_get("cidname", "100").await?;
            channel.database_put("cidname", "100", "Alice").await?;
            channel.database_del("cidname", "100").await?;
            channel.database_deltree("cidname", None).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(DbHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("DATABASE GET");
    assert_eq!(cmd, "DATABASE GET cidname 100");
    client.send_response("200 result=1 (Alice)").await;

    let cmd = client.read_command().await.expect("DATABASE PUT");
    assert_eq!(cmd, "DATABASE PUT cidname 100 Alice");
    client.send_success(1).await;

    let cmd = client.read_command().await.expect("DATABASE DEL");
    assert_eq!(cmd, "DATABASE DEL cidname 100");
    client.send_success(1).await;

    let cmd = client.read_command().await.expect("DATABASE DELTREE");
    assert_eq!(cmd, "DATABASE DELTREE cidname");
    client.send_success(1).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn channel_info_commands() {
    init_tracing();

    struct ChannelInfoHandler;

    impl AgiHandler for ChannelInfoHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.channel_status(None).await?;
            channel.channel_status(Some("SIP/100")).await?;
            channel.wait_for_digit(5000).await?;
            channel.receive_char(3000).await?;
            channel.receive_text(3000).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(ChannelInfoHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("CHANNEL STATUS no arg");
    assert_eq!(cmd, "CHANNEL STATUS");
    client.send_success(6).await;

    let cmd = client
        .read_command()
        .await
        .expect("CHANNEL STATUS with arg");
    assert_eq!(cmd, "CHANNEL STATUS SIP/100");
    client.send_success(6).await;

    let cmd = client.read_command().await.expect("WAIT FOR DIGIT");
    assert_eq!(cmd, "WAIT FOR DIGIT 5000");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("RECEIVE CHAR");
    assert_eq!(cmd, "RECEIVE CHAR 3000");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("RECEIVE TEXT");
    assert_eq!(cmd, "RECEIVE TEXT 3000");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn variable_and_expression_commands() {
    init_tracing();

    struct VarHandler {
        tx: mpsc::Sender<String>,
    }

    impl AgiHandler for VarHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            let resp = channel.get_variable("CHANNEL").await?;
            // capture the data from the response
            if let Some(data) = resp.data {
                let _ = self.tx.send(data).await;
            }
            channel.set_variable("FOO", "bar").await?;
            channel.get_full_variable("${CHANNEL}", None).await?;
            channel
                .get_full_variable("${EXTEN}", Some("SIP/100"))
                .await?;
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<String>(1);
    let (handle, shutdown, addr) = spawn_server(VarHandler { tx }, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // GET VARIABLE CHANNEL
    let cmd = client.read_command().await.expect("GET VARIABLE");
    assert_eq!(cmd, "GET VARIABLE CHANNEL");
    client.send_response("200 result=1 (SIP/100-0001)").await;

    // verify handler captured the response data
    let data = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for captured data")
        .expect("channel closed");
    assert_eq!(data, "SIP/100-0001");

    // SET VARIABLE FOO bar
    let cmd = client.read_command().await.expect("SET VARIABLE");
    assert_eq!(cmd, "SET VARIABLE FOO bar");
    client.send_success(1).await;

    // GET FULL VARIABLE ${CHANNEL} — expression contains special chars
    let cmd = client
        .read_command()
        .await
        .expect("GET FULL VARIABLE no channel");
    assert_eq!(cmd, "GET FULL VARIABLE ${CHANNEL}");
    client.send_response("200 result=1 (SIP/100-0001)").await;

    // GET FULL VARIABLE ${EXTEN} SIP/100
    let cmd = client
        .read_command()
        .await
        .expect("GET FULL VARIABLE with channel");
    assert_eq!(cmd, "GET FULL VARIABLE ${EXTEN} SIP/100");
    client.send_response("200 result=1 (200)").await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn error_response_510_invalid_command() {
    init_tracing();

    struct RawCommandHandler {
        tx: mpsc::Sender<u16>,
    }

    impl AgiHandler for RawCommandHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            let resp = channel.send_command("INVALID_CMD\n").await?;
            let _ = self.tx.send(resp.code).await;
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<u16>(1);
    let (handle, shutdown, addr) = spawn_server(RawCommandHandler { tx }, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client
        .read_command()
        .await
        .expect("should read raw command");
    assert_eq!(cmd, "INVALID_CMD");
    client.send_response("510 Invalid or unknown command").await;

    let code = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for response code")
        .expect("channel closed");
    assert_eq!(code, 510, "handler should receive 510 response code");

    // handler returns Ok, so connection closes normally
    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn error_response_520_usage() {
    init_tracing();

    struct UsageErrorHandler {
        tx: mpsc::Sender<u16>,
    }

    impl AgiHandler for UsageErrorHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            let resp = channel.send_command("EXEC\n").await?;
            let _ = self.tx.send(resp.code).await;
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<u16>(1);
    let (handle, shutdown, addr) = spawn_server(UsageErrorHandler { tx }, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("should read EXEC");
    assert_eq!(cmd, "EXEC");
    client
        .send_response("520 result=-1 Usage: EXEC <app> [<args>]")
        .await;

    let code = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out waiting for response code")
        .expect("channel closed");
    assert_eq!(code, 520, "handler should receive 520 response code");

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn speech_commands() {
    init_tracing();

    struct SpeechHandler;

    impl AgiHandler for SpeechHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.send_text("hello").await?;
            channel.send_image("test.png").await?;
            channel.set_autohangup(30).await?;
            channel.set_context("default").await?;
            channel.set_extension("100").await?;
            channel.set_priority("1").await?;
            channel.set_music(true, None).await?;
            channel.set_music(false, Some("jazz")).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SpeechHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SEND TEXT");
    assert_eq!(cmd, "SEND TEXT hello");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SEND IMAGE");
    assert_eq!(cmd, "SEND IMAGE test.png");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET AUTOHANGUP");
    assert_eq!(cmd, "SET AUTOHANGUP 30");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET CONTEXT");
    assert_eq!(cmd, "SET CONTEXT default");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET EXTENSION");
    assert_eq!(cmd, "SET EXTENSION 100");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET PRIORITY");
    assert_eq!(cmd, "SET PRIORITY 1");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET MUSIC on");
    assert_eq!(cmd, "SET MUSIC on");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET MUSIC off jazz");
    assert_eq!(cmd, "SET MUSIC off jazz");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn gosub_with_and_without_args() {
    init_tracing();

    struct GosubHandler;

    impl AgiHandler for GosubHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.gosub("sub", "s", "1", None).await?;
            channel.gosub("sub", "s", "1", Some("arg1,arg2")).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(GosubHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("GOSUB without args");
    assert_eq!(cmd, "GOSUB sub s 1");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("GOSUB with args");
    assert_eq!(cmd, "GOSUB sub s 1 arg1,arg2");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn record_file_with_options() {
    init_tracing();

    struct RecordHandler;

    impl AgiHandler for RecordHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel
                .record_file("test", "wav", "#", 5000, true, Some(3))
                .await?;
            channel
                .record_file("minimal", "gsm", "", 3000, false, None)
                .await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(RecordHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // full options: beep + silence
    let cmd = client.read_command().await.expect("RECORD FILE full");
    assert_eq!(cmd, "RECORD FILE test wav # 5000 beep s=3");
    client
        .send_response("200 result=0 (dtmf) endpos=5000")
        .await;

    // minimal: no beep, no silence
    let cmd = client.read_command().await.expect("RECORD FILE minimal");
    assert_eq!(cmd, "RECORD FILE minimal gsm  3000");
    client
        .send_response("200 result=0 (timeout) endpos=3000")
        .await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn send_text_command() {
    init_tracing();

    struct SendTextHandler;

    impl AgiHandler for SendTextHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.send_text("hello world").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SendTextHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SEND TEXT");
    assert_eq!(cmd, "SEND TEXT \"hello world\"");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn send_image_command() {
    init_tracing();

    struct SendImageHandler;

    impl AgiHandler for SendImageHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.send_image("logo.png").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SendImageHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SEND IMAGE");
    assert_eq!(cmd, "SEND IMAGE logo.png");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn set_autohangup_command() {
    init_tracing();

    struct AutohangupHandler;

    impl AgiHandler for AutohangupHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.set_autohangup(60).await?;
            channel.set_autohangup(0).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(AutohangupHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SET AUTOHANGUP 60");
    assert_eq!(cmd, "SET AUTOHANGUP 60");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET AUTOHANGUP 0");
    assert_eq!(cmd, "SET AUTOHANGUP 0");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn set_context_command() {
    init_tracing();

    struct SetContextHandler;

    impl AgiHandler for SetContextHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.set_context("ivr-main").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SetContextHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SET CONTEXT");
    assert_eq!(cmd, "SET CONTEXT ivr-main");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn set_extension_command() {
    init_tracing();

    struct SetExtensionHandler;

    impl AgiHandler for SetExtensionHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.set_extension("s").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SetExtensionHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SET EXTENSION");
    assert_eq!(cmd, "SET EXTENSION s");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn set_priority_command() {
    init_tracing();

    struct SetPriorityHandler;

    impl AgiHandler for SetPriorityHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.set_priority("n").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SetPriorityHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SET PRIORITY");
    assert_eq!(cmd, "SET PRIORITY n");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn set_music_command() {
    init_tracing();

    struct SetMusicHandler;

    impl AgiHandler for SetMusicHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.set_music(true, Some("classical")).await?;
            channel.set_music(false, None).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SetMusicHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SET MUSIC on classical");
    assert_eq!(cmd, "SET MUSIC on classical");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("SET MUSIC off");
    assert_eq!(cmd, "SET MUSIC off");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn control_stream_file_command() {
    init_tracing();

    struct ControlStreamHandler;

    impl AgiHandler for ControlStreamHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel
                .control_stream_file("welcome", "#*", Some(5000), Some("#"), Some("*"), Some("8"))
                .await?;
            channel
                .control_stream_file("goodbye", "", None, None, None, None)
                .await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(ControlStreamHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    // full options
    let cmd = client
        .read_command()
        .await
        .expect("CONTROL STREAM FILE full");
    assert!(cmd.starts_with("CONTROL STREAM FILE"), "got: {cmd}");
    assert!(cmd.contains("welcome"), "should contain filename: {cmd}");
    assert!(cmd.contains("5000"), "should contain skipms: {cmd}");
    assert!(cmd.contains("#"), "should contain ff char: {cmd}");
    client.send_response("200 result=0 endpos=5000").await;

    // defaults only
    let cmd = client
        .read_command()
        .await
        .expect("CONTROL STREAM FILE defaults");
    assert!(cmd.starts_with("CONTROL STREAM FILE"), "got: {cmd}");
    assert!(cmd.contains("goodbye"), "should contain filename: {cmd}");
    assert!(cmd.contains("3000"), "default skipms should be 3000: {cmd}");
    client.send_response("200 result=0 endpos=3000").await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn get_option_command() {
    init_tracing();

    struct GetOptionHandler;

    impl AgiHandler for GetOptionHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.get_option("menu", "123", Some(5000)).await?;
            channel.get_option("prompt", "#", None).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(GetOptionHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client
        .read_command()
        .await
        .expect("GET OPTION with timeout");
    assert_eq!(cmd, "GET OPTION menu 123 5000");
    client.send_response("200 result=0 endpos=5000").await;

    let cmd = client
        .read_command()
        .await
        .expect("GET OPTION without timeout");
    assert_eq!(cmd, "GET OPTION prompt #");
    client.send_response("200 result=0 endpos=3000").await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn get_full_variable_command() {
    init_tracing();

    struct FullVarHandler;

    impl AgiHandler for FullVarHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.get_full_variable("${CALLERID(num)}", None).await?;
            channel
                .get_full_variable("${CDR(duration)}", Some("SIP/200"))
                .await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(FullVarHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client
        .read_command()
        .await
        .expect("GET FULL VARIABLE no channel");
    assert_eq!(cmd, "GET FULL VARIABLE ${CALLERID(num)}");
    client.send_response("200 result=1 (100)").await;

    let cmd = client
        .read_command()
        .await
        .expect("GET FULL VARIABLE with channel");
    assert_eq!(cmd, "GET FULL VARIABLE ${CDR(duration)} SIP/200");
    client.send_response("200 result=1 (42)").await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn receive_char_command() {
    init_tracing();

    struct ReceiveCharHandler;

    impl AgiHandler for ReceiveCharHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.receive_char(5000).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(ReceiveCharHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("RECEIVE CHAR");
    assert_eq!(cmd, "RECEIVE CHAR 5000");
    client.send_success(65).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn receive_text_command() {
    init_tracing();

    struct ReceiveTextHandler;

    impl AgiHandler for ReceiveTextHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.receive_text(10000).await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(ReceiveTextHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("RECEIVE TEXT");
    assert_eq!(cmd, "RECEIVE TEXT 10000");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn tdd_mode_command() {
    init_tracing();

    struct TddModeHandler;

    impl AgiHandler for TddModeHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.tdd_mode("on").await?;
            channel.tdd_mode("off").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(TddModeHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("TDD MODE on");
    assert_eq!(cmd, "TDD MODE on");
    client.send_success(1).await;

    let cmd = client.read_command().await.expect("TDD MODE off");
    assert_eq!(cmd, "TDD MODE off");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn asyncagi_break_command() {
    init_tracing();

    struct AsyncAgiBreakHandler;

    impl AgiHandler for AsyncAgiBreakHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.asyncagi_break().await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(AsyncAgiBreakHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("ASYNCAGI BREAK");
    assert_eq!(cmd, "ASYNCAGI BREAK");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn speech_set_command() {
    init_tracing();

    struct SpeechSetHandler;

    impl AgiHandler for SpeechSetHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.speech_set("min_score", "50").await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SpeechSetHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("SPEECH SET");
    assert_eq!(cmd, "SPEECH SET min_score 50");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn noop_command() {
    init_tracing();

    struct NoopHandler;

    impl AgiHandler for NoopHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            channel.noop().await?;
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(NoopHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("NOOP");
    assert_eq!(cmd, "NOOP");
    client.send_success(0).await;

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

// ---------------------------------------------------------------------------
// edge case tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn handler_error_does_not_crash_server() {
    init_tracing();

    struct FailingHandler;

    impl AgiHandler for FailingHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            _channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            Err(AgiError::Io(std::io::Error::other("handler exploded")))
        }
    }

    let (handle, shutdown, addr) = spawn_server(FailingHandler, None).await;
    let env = standard_env();

    // first connection — handler returns Err immediately
    let mut client1 = MockAgiClient::connect(addr, &env).await;
    // handler errors out, connection closes without sending commands
    let eof = client1.read_command().await;
    assert!(eof.is_none(), "connection should close after handler error");

    // small sleep for the server to process the error
    tokio::time::sleep(Duration::from_millis(50)).await;

    // second connection — server should still be alive and accept it
    let mut client2 = MockAgiClient::connect(addr, &env).await;
    let eof = client2.read_command().await;
    assert!(
        eof.is_none(),
        "second connection should also close after handler error"
    );

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly despite handler errors");
}

#[tokio::test]
async fn multiple_commands_in_sequence() {
    init_tracing();

    struct SequentialHandler;

    impl AgiHandler for SequentialHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            for i in 0..12 {
                channel.set_variable("STEP", &i.to_string()).await?;
            }
            Ok(())
        }
    }

    let (handle, shutdown, addr) = spawn_server(SequentialHandler, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    for i in 0..12 {
        let cmd = client
            .read_command()
            .await
            .unwrap_or_else(|| panic!("should read command {i}"));
        assert!(
            cmd.starts_with("SET VARIABLE"),
            "command {i} should be SET VARIABLE, got: {cmd}"
        );
        assert!(
            cmd.contains(&i.to_string()),
            "command {i} should contain value: {cmd}"
        );
        client.send_success(1).await;
    }

    assert!(client.read_command().await.is_none(), "stream should close");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn empty_environment_variables() {
    init_tracing();

    struct EmptyEnvHandler {
        tx: mpsc::Sender<bool>,
    }

    impl AgiHandler for EmptyEnvHandler {
        async fn handle(
            &self,
            request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            // handler should receive a request even with no env vars
            let has_no_channel = request.channel().is_none();
            let _ = self.tx.send(has_no_channel).await;
            channel.answer().await?;
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<bool>(1);
    let (handle, shutdown, addr) = spawn_server(EmptyEnvHandler { tx }, None).await;

    // connect with zero env vars
    let mut client = MockAgiClient::connect(addr, &[]).await;

    let has_no_channel = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert!(has_no_channel, "channel name should be None with empty env");

    let cmd = client.read_command().await.expect("ANSWER");
    assert_eq!(cmd, "ANSWER");
    client.send_success(0).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn large_command_response() {
    init_tracing();

    struct LargeResponseHandler {
        tx: mpsc::Sender<String>,
    }

    impl AgiHandler for LargeResponseHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            let resp = channel.get_variable("LARGE_VAR").await?;
            if let Some(data) = resp.data {
                let _ = self.tx.send(data).await;
            }
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<String>(1);
    let (handle, shutdown, addr) = spawn_server(LargeResponseHandler { tx }, None).await;
    let env = standard_env();
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("GET VARIABLE");
    assert_eq!(cmd, "GET VARIABLE LARGE_VAR");

    // send a response with a very long data field
    let long_value: String = "x".repeat(4096);
    client
        .send_response(&format!("200 result=1 ({long_value})"))
        .await;

    let data = tokio::time::timeout(Duration::from_secs(5), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");
    assert_eq!(data.len(), 4096, "handler should receive full large data");
    assert!(data.chars().all(|c| c == 'x'), "data should be all x's");

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn concurrent_handlers_independent() {
    init_tracing();

    struct IndexedHandler {
        tx: mpsc::Sender<String>,
    }

    impl AgiHandler for IndexedHandler {
        async fn handle(
            &self,
            request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            // echo back the unique_id from the request
            let uid = request.unique_id().unwrap_or("unknown").to_owned();
            channel.set_variable("UID", &uid).await?;
            let _ = self.tx.send(uid).await;
            Ok(())
        }
    }

    let (tx, mut rx) = mpsc::channel::<String>(2);
    let (handle, shutdown, addr) = spawn_server(IndexedHandler { tx }, None).await;

    // two clients with different unique_ids
    let env1: Vec<(&str, &str)> = vec![("channel", "SIP/100-00000001"), ("uniqueid", "aaa.1")];
    let env2: Vec<(&str, &str)> = vec![("channel", "SIP/200-00000002"), ("uniqueid", "bbb.2")];

    let addr2 = addr;
    let t1 = tokio::spawn(async move {
        let mut client = MockAgiClient::connect(addr2, &env1).await;
        let cmd = client
            .read_command()
            .await
            .expect("SET VARIABLE from client1");
        assert!(cmd.starts_with("SET VARIABLE"), "got: {cmd}");
        assert!(
            cmd.contains("aaa.1"),
            "client1 should use its own uid: {cmd}"
        );
        client.send_success(1).await;
    });

    let addr3 = addr;
    let t2 = tokio::spawn(async move {
        let mut client = MockAgiClient::connect(addr3, &env2).await;
        let cmd = client
            .read_command()
            .await
            .expect("SET VARIABLE from client2");
        assert!(cmd.starts_with("SET VARIABLE"), "got: {cmd}");
        assert!(
            cmd.contains("bbb.2"),
            "client2 should use its own uid: {cmd}"
        );
        client.send_success(1).await;
    });

    t1.await.expect("client1 task should not panic");
    t2.await.expect("client2 task should not panic");

    // collect both uid signals
    let mut uids = Vec::new();
    for _ in 0..2 {
        let uid = tokio::time::timeout(Duration::from_secs(5), rx.recv())
            .await
            .expect("timed out waiting for uid")
            .expect("channel closed");
        uids.push(uid);
    }
    uids.sort();
    assert_eq!(uids, vec!["aaa.1", "bbb.2"]);

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}

#[tokio::test]
async fn server_builder_without_handler_fails() {
    init_tracing();

    let result = AgiServer::<AnswerAndHangup>::builder()
        .bind("127.0.0.1:0")
        .build()
        .await;

    assert!(result.is_err(), "building without a handler should fail");
}

#[tokio::test]
async fn server_binds_to_specified_address() {
    init_tracing();

    let port = free_port().await;
    let bind_addr = format!("127.0.0.1:{port}");

    let (server, shutdown) = AgiServer::builder()
        .bind(&bind_addr)
        .handler(AnswerAndHangup)
        .build()
        .await
        .expect("should build server");
    let handle = tokio::spawn(server.run());
    tokio::time::sleep(Duration::from_millis(20)).await;

    // verify we can connect to the specified address
    let env = standard_env();
    let addr: std::net::SocketAddr = bind_addr.parse().expect("valid socket addr");
    let mut client = MockAgiClient::connect(addr, &env).await;

    let cmd = client.read_command().await.expect("ANSWER");
    assert_eq!(cmd, "ANSWER");
    client.send_success(0).await;

    let cmd = client.read_command().await.expect("HANGUP");
    assert_eq!(cmd, "HANGUP");
    client.send_success(1).await;

    shutdown.shutdown();
    let result = handle.await.expect("task should not panic");
    result.expect("server should exit cleanly");
}
