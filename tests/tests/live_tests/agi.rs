use std::time::Duration;

use asterisk_rs_agi::channel::AgiChannel;
use asterisk_rs_agi::handler::AgiHandler;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::server::AgiServer;
use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::*;
use tokio::sync::mpsc;

/// data captured by the test handler
struct AgiSessionCapture {
    channel: Option<String>,
    unique_id: Option<String>,
    extension: Option<String>,
    commands_sent: Vec<String>,
}

/// test handler that captures session data and sends a few commands
struct IntegrationHandler {
    tx: mpsc::Sender<AgiSessionCapture>,
}

impl AgiHandler for IntegrationHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let mut commands_sent = Vec::new();

        // answer the channel
        let resp = channel.answer().await?;
        commands_sent.push(format!("ANSWER -> {}", resp.code));

        // set a variable
        let resp = channel.set_variable("AGI_TEST", "integration").await?;
        commands_sent.push(format!("SET VARIABLE -> {}", resp.code));

        // noop as a ping
        let resp = channel.noop().await?;
        commands_sent.push(format!("NOOP -> {}", resp.code));

        // hangup
        let resp = channel.hangup(None).await?;
        commands_sent.push(format!("HANGUP -> {}", resp.code));

        let _ = self
            .tx
            .send(AgiSessionCapture {
                channel: request.channel().map(String::from),
                unique_id: request.unique_id().map(String::from),
                extension: request.extension().map(String::from),
                commands_sent,
            })
            .await;

        Ok(())
    }
}

#[tokio::test]
async fn real_agi_session() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel::<AgiSessionCapture>(1);
    let handler = IntegrationHandler { tx };

    // bind AGI server on port 4573 (what extensions.conf expects)
    let (server, shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(handler)
        .build()
        .await
        .expect("failed to bind AGI server");

    let server_handle = tokio::spawn(server.run());

    // connect to AMI and originate a call to extension 200 which triggers
    // AGI(agi://host.docker.internal:4573)
    let ami = AmiClient::builder()
        .host(ami_host())
        .port(ami_port())
        .credentials("testadmin", "testsecret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("failed to connect AMI");

    let action = OriginateAction {
        channel: "Local/200@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("200".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(15000),
        caller_id: Some("agi-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success, "originate should be accepted");

    // wait for the AGI handler to complete
    let capture = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out waiting for AGI session")
        .expect("capture channel closed");

    // verify the handler received valid session data
    assert!(
        capture.channel.is_some(),
        "handler should receive channel name"
    );
    assert!(
        capture.unique_id.is_some(),
        "handler should receive unique id"
    );
    assert_eq!(
        capture.extension.as_deref(),
        Some("200"),
        "handler should receive extension 200"
    );

    // verify commands were executed
    assert!(
        !capture.commands_sent.is_empty(),
        "handler should have sent commands"
    );
    // all commands should have returned 200
    for cmd in &capture.commands_sent {
        assert!(cmd.contains("200"), "command should return 200: {cmd}");
    }

    ami.disconnect().await.expect("ami disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// helpers shared across new tests
// ---------------------------------------------------------------------------

async fn connect_ami() -> AmiClient {
    AmiClient::builder()
        .host(ami_host())
        .port(ami_port())
        .credentials("testadmin", "testsecret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("failed to connect AMI")
}

fn originate_agi() -> OriginateAction {
    OriginateAction {
        channel: "Local/200@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("200".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(15000),
        caller_id: Some("agi-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    }
}

async fn spawn_agi<H: AgiHandler + Send + Sync + 'static>(
    handler: H,
) -> (
    tokio::task::JoinHandle<asterisk_rs_agi::error::Result<()>>,
    asterisk_rs_agi::server::ShutdownHandle,
) {
    let (server, shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(handler)
        .build()
        .await
        .expect("failed to bind AGI server");
    let handle = tokio::spawn(server.run());
    (handle, shutdown)
}

// ---------------------------------------------------------------------------
// 1. agi_get_variable
// ---------------------------------------------------------------------------

struct GetVariableHandler {
    tx: mpsc::Sender<(Option<String>, Option<String>)>,
}

impl AgiHandler for GetVariableHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let existing = channel.get_variable("CHANNEL(name)").await?;
        let missing = channel.get_variable("NONEXISTENT_VAR_12345").await?;
        let _ = channel.hangup(None).await;
        let _ = self
            .tx
            .send((existing.data.clone(), missing.data.clone()))
            .await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_get_variable() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(GetVariableHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success, "originate should succeed");

    let (existing, missing) = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(existing.is_some(), "CHANNEL(name) should return a value");
    assert!(
        existing.as_deref().map_or(false, |v| !v.is_empty()),
        "CHANNEL(name) should be non-empty"
    );
    // nonexistent var returns no data or empty data
    assert!(
        missing.is_none() || missing.as_deref() == Some(""),
        "nonexistent var should be empty, got: {missing:?}"
    );

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 2. agi_set_and_get_variable
// ---------------------------------------------------------------------------

struct SetGetVariableHandler {
    tx: mpsc::Sender<Option<String>>,
}

impl AgiHandler for SetGetVariableHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let set_resp = channel.set_variable("MY_AGI_VAR", "test_value").await?;
        assert_eq!(set_resp.code, 200, "set_variable should return 200");
        let get_resp = channel.get_variable("MY_AGI_VAR").await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(get_resp.data.clone()).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_set_and_get_variable() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(SetGetVariableHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let value = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(
        value.as_deref(),
        Some("test_value"),
        "get should return what was set"
    );

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 3. agi_database_operations
// ---------------------------------------------------------------------------

struct DatabaseOpsResult {
    put_ok: bool,
    get_value: Option<String>,
    del_ok: bool,
    get_after_del: Option<String>,
}

struct DatabaseOpsHandler {
    tx: mpsc::Sender<DatabaseOpsResult>,
}

impl AgiHandler for DatabaseOpsHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let put = channel.database_put("agi_test_3", "key1", "value1").await?;
        let get1 = channel.database_get("agi_test_3", "key1").await?;
        let del = channel.database_del("agi_test_3", "key1").await?;
        // after delete, get should fail (result = 0 or no data)
        let get2 = channel.database_get("agi_test_3", "key1").await;
        let get2_data = get2.ok().and_then(|r| r.data);
        let _ = channel.hangup(None).await;
        let _ = self
            .tx
            .send(DatabaseOpsResult {
                put_ok: put.code == 200 && put.result == 1,
                get_value: get1.data,
                del_ok: del.code == 200 && del.result == 1,
                get_after_del: get2_data,
            })
            .await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_database_operations() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(DatabaseOpsHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let result = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(result.put_ok, "database_put should succeed");
    assert_eq!(
        result.get_value.as_deref(),
        Some("value1"),
        "database_get should return the stored value"
    );
    assert!(result.del_ok, "database_del should succeed");
    assert!(
        result.get_after_del.is_none() || result.get_after_del.as_deref() == Some(""),
        "database_get after delete should fail or be empty"
    );

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 4. agi_channel_status
// ---------------------------------------------------------------------------

struct ChannelStatusHandler {
    tx: mpsc::Sender<(u16, i32)>,
}

impl AgiHandler for ChannelStatusHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let resp = channel.channel_status(None).await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send((resp.code, resp.result)).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_channel_status() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(ChannelStatusHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let (code, result) = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(code, 200, "channel_status should return 200");
    // 6 = channel is up/answered
    assert_eq!(result, 6, "channel should be in answered state");

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 5. agi_exec_application
// ---------------------------------------------------------------------------

struct ExecAppHandler {
    tx: mpsc::Sender<u16>,
}

impl AgiHandler for ExecAppHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let resp = channel.exec("NoOp", "test_exec_arg").await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(resp.code).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_exec_application() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(ExecAppHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let code = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(code, 200, "exec NoOp should return 200");

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 6. agi_verbose
// ---------------------------------------------------------------------------

struct VerboseHandler {
    tx: mpsc::Sender<u16>,
}

impl AgiHandler for VerboseHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let resp = channel.verbose("integration test message", 1).await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(resp.code).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_verbose() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(VerboseHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let code = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(code, 200, "verbose should return 200");

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 7. agi_noop
// ---------------------------------------------------------------------------

struct NoopHandler {
    tx: mpsc::Sender<u16>,
}

impl AgiHandler for NoopHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let resp = channel.noop().await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(resp.code).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_noop() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(NoopHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let code = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(code, 200, "noop should return 200");

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 8. agi_set_callerid
// ---------------------------------------------------------------------------

struct SetCallerIdHandler {
    tx: mpsc::Sender<(u16, Option<String>)>,
}

impl AgiHandler for SetCallerIdHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let set_resp = channel.set_callerid("\"Test Name\" <5551234>").await?;
        let get_resp = channel.get_variable("CALLERID(num)").await?;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send((set_resp.code, get_resp.data)).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_set_callerid() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(SetCallerIdHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let (code, callerid_num) = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(code, 200, "set_callerid should return 200");
    assert_eq!(
        callerid_num.as_deref(),
        Some("5551234"),
        "caller id number should be set"
    );

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 9. agi_request_metadata
// ---------------------------------------------------------------------------

struct RequestMetadata {
    channel: Option<String>,
    unique_id: Option<String>,
    context: Option<String>,
    extension: Option<String>,
    priority: Option<String>,
    caller_id: Option<String>,
    language: Option<String>,
}

struct RequestMetadataHandler {
    tx: mpsc::Sender<RequestMetadata>,
}

impl AgiHandler for RequestMetadataHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let meta = RequestMetadata {
            channel: request.channel().map(String::from),
            unique_id: request.unique_id().map(String::from),
            context: request.context().map(String::from),
            extension: request.extension().map(String::from),
            priority: request.priority().map(String::from),
            caller_id: request.caller_id().map(String::from),
            language: request.language().map(String::from),
        };
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(meta).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_request_metadata() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(RequestMetadataHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let meta = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert!(
        meta.channel.as_ref().map_or(false, |c| !c.is_empty()),
        "channel should be present"
    );
    assert!(
        meta.unique_id.as_ref().map_or(false, |u| !u.is_empty()),
        "unique_id should be present"
    );
    assert_eq!(
        meta.context.as_deref(),
        Some("default"),
        "context should be default"
    );
    assert_eq!(
        meta.extension.as_deref(),
        Some("200"),
        "extension should be 200"
    );
    assert!(
        meta.priority.as_ref().map_or(false, |p| !p.is_empty()),
        "priority should be present"
    );
    assert!(
        meta.language.as_ref().map_or(false, |l| !l.is_empty()),
        "language should be present"
    );
    assert!(
        meta.caller_id.as_ref().map_or(false, |c| !c.is_empty()),
        "caller_id should be present"
    );

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 10. agi_multiple_commands_sequence
// ---------------------------------------------------------------------------

struct MultiCommandHandler {
    tx: mpsc::Sender<Vec<u16>>,
}

impl AgiHandler for MultiCommandHandler {
    async fn handle(
        &self,
        _request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let mut codes = Vec::new();

        codes.push(channel.answer().await?.code);
        codes.push(channel.set_variable("SEQ_A", "1").await?.code);
        codes.push(channel.set_variable("SEQ_B", "2").await?.code);
        codes.push(channel.set_variable("SEQ_C", "3").await?.code);
        codes.push(channel.get_variable("SEQ_A").await?.code);
        codes.push(channel.get_variable("SEQ_B").await?.code);
        codes.push(channel.get_variable("SEQ_C").await?.code);
        codes.push(channel.noop().await?.code);
        codes.push(channel.verbose("sequence test", 1).await?.code);
        codes.push(channel.hangup(None).await?.code);

        let _ = self.tx.send(codes).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_multiple_commands_sequence() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(1);
    let (server_handle, shutdown) = spawn_agi(MultiCommandHandler { tx }).await;
    let ami = connect_ami().await;

    let resp = ami
        .send_action(&originate_agi())
        .await
        .expect("originate failed");
    assert!(resp.success);

    let codes = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out")
        .expect("channel closed");

    assert_eq!(codes.len(), 10, "should have 10 command responses");
    for (i, code) in codes.iter().enumerate() {
        assert_eq!(*code, 200, "command {i} should return 200, got {code}");
    }

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

// ---------------------------------------------------------------------------
// 11. agi_concurrent_sessions
// ---------------------------------------------------------------------------

struct ConcurrentSessionHandler {
    tx: mpsc::Sender<String>,
}

impl AgiHandler for ConcurrentSessionHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        let uid = request.unique_id().unwrap_or("unknown").to_string();
        let _ = channel.noop().await;
        let _ = channel.hangup(None).await;
        let _ = self.tx.send(uid).await;
        Ok(())
    }
}

#[tokio::test]
async fn agi_concurrent_sessions() {
    init_tracing();

    let (tx, mut rx) = mpsc::channel(4);
    let (server_handle, shutdown) = spawn_agi(ConcurrentSessionHandler { tx }).await;
    let ami = connect_ami().await;

    // originate two calls concurrently
    let resp1 = ami
        .send_action(&originate_agi())
        .await
        .expect("originate 1 failed");
    let resp2 = ami
        .send_action(&originate_agi())
        .await
        .expect("originate 2 failed");
    assert!(resp1.success, "originate 1 should succeed");
    assert!(resp2.success, "originate 2 should succeed");

    // collect both unique_ids
    let uid1 = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out waiting for session 1")
        .expect("channel closed");
    let uid2 = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out waiting for session 2")
        .expect("channel closed");

    assert_ne!(uid1, uid2, "two sessions should have distinct unique_ids");

    ami.disconnect().await.expect("disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}
