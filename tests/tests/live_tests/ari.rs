use std::time::Duration;

use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::{AriClient, AriEvent};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::*;

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
        .expect("failed to connect ARI client")
}

/// build an AMI client for triggering calls
async fn connect_ami() -> AmiClient {
    AmiClient::builder()
        .host(ami_host())
        .port(ami_port())
        .credentials("testadmin", "testsecret")
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("failed to connect AMI client")
}

// ---------------------------------------------------------------------------
// tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn connect_and_get_asterisk_info() {
    init_tracing();

    let client = connect_ari().await;

    // give WS time to connect
    tokio::time::sleep(Duration::from_millis(500)).await;

    let info: serde_json::Value = client
        .get("asterisk/info")
        .await
        .expect("GET asterisk/info failed");

    // the response should contain system and config sections
    assert!(
        info.get("system").is_some() || info.get("config").is_some(),
        "asterisk/info should have system or config sections: {info}"
    );

    client.disconnect();
}

#[tokio::test]
async fn list_channels() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let channels: Vec<serde_json::Value> =
        client.get("channels").await.expect("GET channels failed");

    // may be empty, but should not error
    // just verify we got a valid array back
    tracing::info!(count = channels.len(), "active channels");

    client.disconnect();
}

#[tokio::test]
async fn list_bridges() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let bridges: Vec<serde_json::Value> = client.get("bridges").await.expect("GET bridges failed");

    tracing::info!(count = bridges.len(), "active bridges");

    client.disconnect();
}

#[tokio::test]
async fn stasis_event_from_originate() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();

    // give WS time to connect and subscribe
    tokio::time::sleep(Duration::from_secs(1)).await;

    // originate directly into Stasis application (bypasses dialplan)
    let ami = connect_ami().await;

    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("ari-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // wait for StasisStart on the ARI event stream
    let event = tokio::time::timeout(Duration::from_secs(15), async {
        loop {
            let msg = sub.recv().await.expect("ari event bus closed");
            tracing::info!(event = ?msg.event, app = %msg.application, "ari event received");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                // match our originate by caller id
                if channel.caller.number == "100" {
                    return msg;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart event");

    assert_eq!(event.application, "test-app");
    if let AriEvent::StasisStart { channel, .. } = &event.event {
        assert!(!channel.id.is_empty(), "channel id should not be empty");
        tracing::info!(
            channel_id = %channel.id,
            channel_name = %channel.name,
            "received StasisStart"
        );

        // hangup the channel via ARI to clean up
        let _ = ari.delete(&format!("channels/{}", channel.id)).await;
    }

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_answer_and_hangup_via_ari() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // originate into Stasis without pre-answering (ext 301)
    let ami = connect_ami().await;
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("ari-answer-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(
        response.success,
        "originate should be accepted: {response:?}"
    );

    // wait for StasisStart
    let channel_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                if channel.caller.number == "100" {
                    return channel.id.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart");

    // answer the channel via ARI REST
    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer via ARI failed");

    // verify channel state changed
    tokio::time::sleep(Duration::from_millis(200)).await;

    // hangup via ARI REST
    ari.delete(&format!("channels/{channel_id}"))
        .await
        .expect("hangup via ARI failed");

    // should see StasisEnd event
    let saw_end = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisEnd { channel, .. } = &msg.event {
                if channel.id == channel_id {
                    return true;
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisEnd");

    assert!(saw_end);

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn bridge_create_add_channels_destroy() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;

    // originate two channels into Stasis
    let mut channel_ids = Vec::new();
    for i in 0..2 {
        let action = OriginateAction {
            channel: "Local/999@default".to_string(),
            context: None,
            exten: None,
            priority: None,
            application: Some("Stasis".to_string()),
            data: Some("test-app".to_string()),
            timeout: Some(10000),
            caller_id: Some(format!("bridge-test-{i} <200>")),
            account: None,
            async_: true,
            variables: vec![],
        };
        let response = ami.send_action(&action).await.expect("originate failed");
        assert!(response.success);
    }

    // collect both StasisStart events
    for _ in 0..2 {
        let cid = tokio::time::timeout(Duration::from_secs(10), async {
            loop {
                let msg = sub.recv().await.expect("event bus closed");
                if let AriEvent::StasisStart { channel, .. } = &msg.event {
                    if channel.caller.number == "200" {
                        return channel.id.clone();
                    }
                }
            }
        })
        .await
        .expect("timed out waiting for StasisStart");
        channel_ids.push(cid);
    }

    // create a bridge via ARI
    let bridge: serde_json::Value = ari
        .post("bridges", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("create bridge failed");
    let bridge_id = bridge["id"]
        .as_str()
        .expect("bridge should have id")
        .to_string();

    // add both channels to the bridge
    for cid in &channel_ids {
        ari.post_empty(&format!("bridges/{bridge_id}/addChannel?channel={cid}"))
            .await
            .expect("add channel to bridge failed");
    }

    // verify ChannelEnteredBridge events
    let mut entered_count = 0;
    let _ = tokio::time::timeout(Duration::from_secs(5), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
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

    // destroy the bridge (auto-kicks channels)
    ari.delete(&format!("bridges/{bridge_id}"))
        .await
        .expect("destroy bridge failed");

    // cleanup: hangup channels
    for cid in &channel_ids {
        let _ = ari.delete(&format!("channels/{cid}")).await;
    }

    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_dtmf_via_ari() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("dtmf-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    // wait for StasisStart
    let channel_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                if channel.caller.number == "100" {
                    return channel.id.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart");

    // send DTMF via ARI
    ari.post_empty(&format!("channels/{channel_id}/dtmf?dtmf=1234"))
        .await
        .expect("send dtmf failed");

    // DTMF events may or may not fire back to the Stasis app depending on
    // channel type and direction. the important assertion is that the REST
    // call above succeeded (line 384). collect any events opportunistically.
    let mut digits = Vec::new();
    let _ = tokio::time::timeout(Duration::from_secs(3), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::ChannelDtmfReceived { channel, digit, .. } = &msg.event {
                if channel.id == channel_id {
                    digits.push(digit.clone());
                    if digits.len() >= 4 {
                        break;
                    }
                }
            }
        }
    })
    .await;

    tracing::info!(digits = ?digits, "collected DTMF events (may be empty for Local channels)");

    // cleanup
    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_variable_via_ari() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("var-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };

    let response = ami.send_action(&action).await.expect("originate failed");
    assert!(response.success);

    let channel_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                if channel.caller.number == "100" {
                    return channel.id.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart");

    // set a variable via ARI
    ari.post_empty(&format!(
        "channels/{channel_id}/variable?variable=MY_ARI_VAR&value=ari_value"
    ))
    .await
    .expect("set variable failed");

    // get it back
    let var: serde_json::Value = ari
        .get(&format!(
            "channels/{channel_id}/variable?variable=MY_ARI_VAR"
        ))
        .await
        .expect("get variable failed");

    assert_eq!(
        var["value"].as_str(),
        Some("ari_value"),
        "variable value should match: {var}"
    );

    // cleanup
    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}
