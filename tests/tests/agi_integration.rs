#![cfg(feature = "integration")]

mod common;

use std::time::Duration;

use asterisk_rs_agi::channel::AgiChannel;
use asterisk_rs_agi::handler::AgiHandler;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::server::AgiServer;
use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
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
    common::init_tracing();

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
        .host(common::ami_host())
        .port(common::ami_port())
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
