#![cfg(feature = "integration")]

mod common;

use std::time::Duration;

use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::{AriClient, AriEvent};
use asterisk_rs_core::config::ReconnectPolicy;

/// build an ARI client connected to the test Asterisk instance
async fn connect_ari() -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host(common::ari_host())
        .port(common::ari_port())
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
        .host(common::ami_host())
        .port(common::ami_port())
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
    common::init_tracing();

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
    common::init_tracing();

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
    common::init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let bridges: Vec<serde_json::Value> = client.get("bridges").await.expect("GET bridges failed");

    tracing::info!(count = bridges.len(), "active bridges");

    client.disconnect();
}

#[tokio::test]
async fn stasis_event_from_originate() {
    common::init_tracing();

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
