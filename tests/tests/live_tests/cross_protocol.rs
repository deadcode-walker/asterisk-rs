// cross-protocol integration tests exercising AMI + ARI + AGI together

use std::time::Duration;

use asterisk_rs_agi::channel::AgiChannel;
use asterisk_rs_agi::handler::AgiHandler;
use asterisk_rs_agi::request::AgiRequest;
use asterisk_rs_agi::server::AgiServer;
use asterisk_rs_ami::action::{GetVarAction, OriginateAction};
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ami::AmiEvent;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::{AriClient, AriEvent};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::*;
use tokio::sync::mpsc;

/// build an AMI client connected to the test Asterisk instance
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

/// build an ARI client connected to the test Asterisk instance
async fn connect_ari() -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host(ari_host())
        .port(ari_port())
        .username("testuser")
        .password("testpass")
        .reconnect(ReconnectPolicy::exponential(
            Duration::from_millis(500),
            Duration::from_secs(5),
        ))
        .build()
        .expect("failed to build ARI config");

    AriClient::connect(config)
        .await
        .expect("failed to connect ARI")
}

/// wait for a StasisStart event matching the given caller number, return channel id and name
async fn wait_stasis_start(
    sub: &mut asterisk_rs_core::event::EventSubscription<asterisk_rs_ari::AriMessage>,
    caller_number: &str,
    timeout_secs: u64,
) -> (String, String) {
    tokio::time::timeout(Duration::from_secs(timeout_secs), async {
        loop {
            let msg = sub.recv().await.expect("ari event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                if channel.caller.number == caller_number {
                    return (channel.id.clone(), channel.name.clone());
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn ami_originate_ari_stasis_lifecycle() {
    init_tracing();

    let ari = connect_ari().await;
    let mut ari_sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let mut ami_sub = ami.subscribe();

    // originate into Stasis via AMI
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("lifecycle-test <300>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // ARI sees StasisStart
    let (channel_id, channel_name) = wait_stasis_start(&mut ari_sub, "300", 15).await;
    tracing::info!(%channel_id, %channel_name, "StasisStart received");

    // answer via ARI REST
    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer via ARI failed");

    // hangup via ARI REST
    ari.delete(&format!("channels/{channel_id}"))
        .await
        .expect("hangup via ARI failed");

    // ARI sees StasisEnd
    let saw_stasis_end = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = ari_sub.recv().await.expect("ari event bus closed");
            if let AriEvent::StasisEnd { channel, .. } = &msg.event {
                if channel.id == channel_id {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisEnd");
    assert!(saw_stasis_end);

    // AMI should see a Hangup event for a channel matching the originate
    let saw_hangup = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = ami_sub.recv().await.expect("ami event bus closed");
            if let AmiEvent::Hangup { channel, .. } = &event {
                if channel.contains("Local/") {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for AMI Hangup");
    assert!(saw_hangup);

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn ami_originate_agi_handler_completes() {
    init_tracing();

    // set up AGI handler that proves cross-protocol interaction
    let (tx, mut rx) = mpsc::channel::<Vec<String>>(1);

    struct CrossHandler {
        tx: mpsc::Sender<Vec<String>>,
    }

    impl AgiHandler for CrossHandler {
        async fn handle(
            &self,
            _request: AgiRequest,
            mut channel: AgiChannel,
        ) -> asterisk_rs_agi::error::Result<()> {
            let mut cmds = Vec::new();

            let resp = channel.answer().await?;
            cmds.push(format!("ANSWER -> {}", resp.code));

            let resp = channel.set_variable("AGI_PROOF", "cross_protocol").await?;
            cmds.push(format!("SET VARIABLE -> {}", resp.code));

            let resp = channel.noop().await?;
            cmds.push(format!("NOOP -> {}", resp.code));

            let resp = channel.hangup(None).await?;
            cmds.push(format!("HANGUP -> {}", resp.code));

            let _ = self.tx.send(cmds).await;
            Ok(())
        }
    }

    let handler = CrossHandler { tx };
    let (server, shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(handler)
        .build()
        .await
        .expect("failed to bind AGI server");

    let server_handle = tokio::spawn(server.run());

    // connect AMI and subscribe to events
    let ami = connect_ami().await;
    let mut ami_sub = ami.subscribe();

    // originate to ext 200 which triggers AGI
    let action = OriginateAction {
        channel: "Local/200@default".to_string(),
        context: Some("default".to_string()),
        exten: Some("200".to_string()),
        priority: Some(1),
        application: None,
        data: None,
        timeout: Some(15000),
        caller_id: Some("agi-cross <400>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success, "originate should be accepted");

    // wait for AGI handler to complete
    let cmds = tokio::time::timeout(Duration::from_secs(30), rx.recv())
        .await
        .expect("timed out waiting for AGI session")
        .expect("agi capture channel closed");

    // verify handler sent all four commands successfully
    assert_eq!(cmds.len(), 4, "handler should send 4 commands");
    for cmd in &cmds {
        assert!(cmd.contains("200"), "command should return 200: {cmd}");
    }

    // verify AMI saw lifecycle events: at least Newchannel and Hangup
    let mut saw_new = false;
    let mut saw_hangup = false;
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = ami_sub.recv().await.expect("ami event bus closed");
            match &event {
                AmiEvent::NewChannel { channel, .. } if channel.contains("Local/") => {
                    saw_new = true;
                }
                AmiEvent::Hangup { channel, .. } if channel.contains("Local/") => {
                    saw_hangup = true;
                }
                _ => {}
            }
            if saw_new && saw_hangup {
                break;
            }
        }
    })
    .await;

    assert!(saw_new, "AMI should see NewChannel");
    assert!(saw_hangup, "AMI should see Hangup");

    ami.disconnect().await.expect("ami disconnect failed");
    shutdown.shutdown();
    let _ = server_handle.await;
}

#[tokio::test]
async fn ami_events_during_ari_channel_operations() {
    init_tracing();

    let ami = connect_ami().await;
    let _ami_sub = ami.subscribe();

    let ari = connect_ari().await;
    let mut ari_sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // originate into Stasis
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("cross-var <500>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    // wait for StasisStart — get channel id and name
    let (channel_id, channel_name) = wait_stasis_start(&mut ari_sub, "500", 15).await;
    tracing::info!(%channel_id, %channel_name, "stasis channel ready");

    // answer via ARI
    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // set a variable via ARI REST
    ari.post_empty(&format!(
        "channels/{channel_id}/variable?variable=ARI_SET_VAR&value=from_ari"
    ))
    .await
    .expect("set variable via ARI failed");

    // read it back via AMI GetVar using the channel name from StasisStart
    let get_var = GetVarAction {
        channel: Some(channel_name.clone()),
        variable: "ARI_SET_VAR".to_string(),
    };
    let var_response = ami.send_action(&get_var).await.expect("GetVar failed");
    assert!(
        var_response.success,
        "GetVar should succeed: {var_response:?}"
    );

    let value = var_response.get("Value").unwrap_or_default();
    assert_eq!(value, "from_ari", "AMI should read variable set by ARI");

    // cleanup
    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn ari_bridge_with_two_channels() {
    init_tracing();

    let ari = connect_ari().await;
    let mut ari_sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;

    // originate two channels into Stasis
    for i in 0..2 {
        let action = OriginateAction {
            channel: "Local/999@default".to_string(),
            context: None,
            exten: None,
            priority: None,
            application: Some("Stasis".to_string()),
            data: Some("test-app".to_string()),
            timeout: Some(10000),
            caller_id: Some(format!("bridge-cross-{i} <600>")),
            account: None,
            async_: true,
            variables: vec![],
        };
        let response = ami.send_action(&action).await.expect("originate failed");
        assert!(response.success);
    }

    // collect both StasisStart events
    let mut channel_ids = Vec::new();
    for _ in 0..2 {
        let (cid, _) = wait_stasis_start(&mut ari_sub, "600", 15).await;
        channel_ids.push(cid);
    }
    tracing::info!(?channel_ids, "both channels in stasis");

    // answer both channels
    for cid in &channel_ids {
        ari.post_empty(&format!("channels/{cid}/answer"))
            .await
            .expect("answer failed");
    }

    // create bridge
    let bridge: serde_json::Value = ari
        .post("bridges", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("create bridge failed");
    let bridge_id = bridge["id"]
        .as_str()
        .expect("bridge should have id")
        .to_string();
    tracing::info!(%bridge_id, "bridge created");

    // add both channels to bridge
    for cid in &channel_ids {
        ari.post_empty(&format!("bridges/{bridge_id}/addChannel?channel={cid}"))
            .await
            .expect("add channel to bridge failed");
    }

    // wait for both ChannelEnteredBridge events
    let mut entered_count = 0;
    let _ = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = ari_sub.recv().await.expect("ari event bus closed");
            if let AriEvent::ChannelEnteredBridge { bridge, .. } = &msg.event {
                if bridge.id == bridge_id {
                    entered_count += 1;
                    if entered_count >= 2 {
                        break;
                    }
                }
            }
        }
    })
    .await;
    assert_eq!(entered_count, 2, "both channels should enter bridge");

    // remove first channel from bridge
    ari.post_empty(&format!(
        "bridges/{bridge_id}/removeChannel?channel={}",
        channel_ids[0]
    ))
    .await
    .expect("remove channel from bridge failed");

    // verify ChannelLeftBridge
    let saw_left = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = ari_sub.recv().await.expect("ari event bus closed");
            if let AriEvent::ChannelLeftBridge {
                bridge, channel, ..
            } = &msg.event
            {
                if bridge.id == bridge_id && channel.id == channel_ids[0] {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for ChannelLeftBridge");
    assert!(saw_left);

    // destroy bridge (auto-kicks remaining channel)
    ari.delete(&format!("bridges/{bridge_id}"))
        .await
        .expect("destroy bridge failed");

    // hangup both channels
    for cid in &channel_ids {
        let _ = ari.delete(&format!("channels/{cid}")).await;
    }

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn ami_monitors_ari_activity() {
    init_tracing();

    let ami = connect_ami().await;
    let mut ami_sub = ami.subscribe();

    let ari = connect_ari().await;
    let mut ari_sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // originate into Stasis
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("monitor-test <700>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    // wait for StasisStart
    let (channel_id, channel_name) = wait_stasis_start(&mut ari_sub, "700", 15).await;
    tracing::info!(%channel_id, %channel_name, "stasis channel for monitoring");

    // drain any AMI events accumulated so far, collecting NewChannel
    let mut saw_new_channel = false;
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let event = ami_sub.recv().await.expect("ami event bus closed");
            if let AmiEvent::NewChannel { channel, .. } = &event {
                if channel.contains("Local/") {
                    saw_new_channel = true;
                    break;
                }
            }
        }
    })
    .await;
    assert!(saw_new_channel, "AMI should see NewChannel from originate");

    // answer via ARI
    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer via ARI failed");
    tokio::time::sleep(Duration::from_millis(500)).await;

    // mute via ARI
    ari.post_empty(&format!("channels/{channel_id}/mute?direction=both"))
        .await
        .expect("mute via ARI failed");
    tokio::time::sleep(Duration::from_millis(300)).await;

    // unmute via ARI
    ari.delete(&format!("channels/{channel_id}/mute?direction=both"))
        .await
        .expect("unmute via ARI failed");
    tokio::time::sleep(Duration::from_millis(300)).await;

    // hangup via ARI
    ari.delete(&format!("channels/{channel_id}"))
        .await
        .expect("hangup via ARI failed");

    // AMI should see Hangup for a Local channel
    let saw_hangup = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let event = ami_sub.recv().await.expect("ami event bus closed");
            if let AmiEvent::Hangup { channel, .. } = &event {
                if channel.contains("Local/") {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for AMI Hangup after ARI operations");
    assert!(saw_hangup, "AMI should see Hangup after ARI hangup");

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}
