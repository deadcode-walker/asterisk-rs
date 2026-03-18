mod common;
mod mock;

use std::time::Duration;

use asterisk_rs_agi::channel::AgiChannel;
use asterisk_rs_agi::error::AgiError;
use asterisk_rs_agi::handler::AgiHandler;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::server::AgiServer;
use mock::agi_client::{free_port, standard_env, MockAgiClient};
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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

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
