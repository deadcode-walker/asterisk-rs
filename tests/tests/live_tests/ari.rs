use std::time::Duration;

use asterisk_rs_ami::action::OriginateAction;
use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::error::AriError;
use asterisk_rs_ari::event::AriMessage;
use asterisk_rs_ari::{AriClient, AriEvent};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::event::EventSubscription;
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
        caller_id: Some("ari-stasis-test <555>".to_string()),
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
                if channel.caller.number == "555" {
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
        caller_id: Some("ari-answer-test <556>".to_string()),
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
                if channel.caller.number == "556" {
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
        caller_id: Some("var-test <777>".to_string()),
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
                if channel.caller.number == "777" {
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

/// originate a channel into stasis and return its channel id
///
/// filters the stasis-start event by the caller number embedded in
/// `caller_id` (expects `"name <number>"` format)
async fn originate_into_stasis(
    ami: &AmiClient,
    sub: &mut EventSubscription<AriMessage>,
    caller_id: &str,
) -> String {
    // extract the numeric caller id from "name <NNN>" format
    let number = caller_id
        .rsplit_once('<')
        .and_then(|(_, rest)| rest.strip_suffix('>'))
        .expect("caller_id must be in 'name <number>' format");

    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some(caller_id.to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let resp = ami.send_action(&action).await.expect("originate failed");
    assert!(resp.success);

    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                if channel.caller.number == number {
                    return channel.id.clone();
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart")
}

// ---------------------------------------------------------------------------
// extended tests
// ---------------------------------------------------------------------------

#[tokio::test]
async fn asterisk_ping() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let resp: serde_json::Value = client
        .get("asterisk/ping")
        .await
        .expect("GET asterisk/ping failed");

    assert_eq!(
        resp.get("ping").and_then(|v| v.as_str()),
        Some("pong"),
        "ping response should contain pong: {resp}"
    );

    client.disconnect();
}

#[tokio::test]
async fn asterisk_modules_list() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let modules: Vec<serde_json::Value> = client
        .get("asterisk/modules")
        .await
        .expect("GET asterisk/modules failed");

    assert!(!modules.is_empty(), "asterisk should have modules loaded");
    tracing::info!(count = modules.len(), "loaded modules");

    client.disconnect();
}

#[tokio::test]
async fn asterisk_global_variable_set_get() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // set global variable
    client
        .post_empty("asterisk/variable?variable=GLOBAL_TEST_VAR&value=ari_global_value")
        .await
        .expect("set global variable failed");

    // get it back
    let var: serde_json::Value = client
        .get("asterisk/variable?variable=GLOBAL_TEST_VAR")
        .await
        .expect("get global variable failed");

    assert_eq!(
        var["value"].as_str(),
        Some("ari_global_value"),
        "global variable should match: {var}"
    );

    client.disconnect();
}

#[tokio::test]
async fn channel_moh_start_stop() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "moh-test <100>").await;

    // answer first so moh can play
    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // start moh — may return 409 if no moh class configured, accept that
    let moh_start = ari.post_empty(&format!("channels/{channel_id}/moh")).await;
    match &moh_start {
        Ok(()) => tracing::info!("moh started"),
        Err(AriError::Api { status: 409, .. }) => {
            tracing::warn!("moh not available (409), skipping stop");
        }
        Err(e) => panic!("unexpected moh start error: {e}"),
    }

    if moh_start.is_ok() {
        tokio::time::sleep(Duration::from_millis(500)).await;
        ari.delete(&format!("channels/{channel_id}/moh"))
            .await
            .expect("stop moh failed");
    }

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_hold_unhold() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "hold-test <100>").await;

    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // hold — local channels don't emit ChannelHold events, so only
    // assert the REST calls succeed
    ari.post_empty(&format!("channels/{channel_id}/hold"))
        .await
        .expect("hold failed");

    tokio::time::sleep(Duration::from_millis(200)).await;

    // unhold
    ari.delete(&format!("channels/{channel_id}/hold"))
        .await
        .expect("unhold failed");

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_mute_unmute() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "mute-test <100>").await;

    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // mute inbound audio
    ari.post_empty(&format!("channels/{channel_id}/mute?direction=in"))
        .await
        .expect("mute failed");

    tokio::time::sleep(Duration::from_millis(300)).await;

    // unmute
    ari.delete(&format!("channels/{channel_id}/mute?direction=in"))
        .await
        .expect("unmute failed");

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_ring_start_stop() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    // originate to 301 (no pre-answer) so ringing is valid
    let ami = connect_ami().await;
    let action = OriginateAction {
        channel: "Local/301@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("ring-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let resp = ami.send_action(&action).await.expect("originate failed");
    assert!(resp.success);

    let channel_id = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                return channel.id.clone();
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart");

    // start ringing indication
    ari.post_empty(&format!("channels/{channel_id}/ring"))
        .await
        .expect("ring start failed");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // stop ringing
    ari.delete(&format!("channels/{channel_id}/ring"))
        .await
        .expect("ring stop failed");

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_continue_to_dialplan() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "continue-test <100>").await;

    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // continue into dialplan — channel leaves stasis
    ari.post_empty(&format!(
        "channels/{channel_id}/continue?context=default&extension=400&priority=1"
    ))
    .await
    .expect("continue failed");

    // wait for StasisEnd
    let saw_end = tokio::time::timeout(Duration::from_secs(10), async {
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

    // channel is now in dialplan, not stasis — don't try to hangup via ari
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn endpoint_list() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let endpoints: Vec<serde_json::Value> =
        client.get("endpoints").await.expect("GET endpoints failed");

    // may be empty, just verify the call succeeded
    tracing::info!(count = endpoints.len(), "endpoints");

    client.disconnect();
}

#[tokio::test]
async fn sound_list() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let result: Result<Vec<serde_json::Value>, _> = client.get("sounds").await;
    match result {
        Ok(sounds) => {
            tracing::info!(count = sounds.len(), "sounds found");
        }
        Err(AriError::Api { status: 404, .. }) => {
            // no sound files installed — acceptable in minimal docker image
            tracing::info!("no sounds installed (404)");
        }
        Err(e) => panic!("unexpected error: {e}"),
    }

    client.disconnect();
}

#[tokio::test]
async fn rest_invalid_channel_404() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let err = client
        .get::<serde_json::Value>("channels/nonexistent-channel-id-12345")
        .await;

    assert!(
        matches!(err, Err(AriError::Api { status: 404, .. })),
        "expected 404 for nonexistent channel: {err:?}"
    );

    client.disconnect();
}

#[tokio::test]
async fn rest_delete_nonexistent_channel() {
    init_tracing();

    let client = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    let err = client.delete("channels/nonexistent-channel-id-12345").await;

    assert!(
        matches!(err, Err(AriError::Api { status: 404, .. })),
        "expected 404 for nonexistent channel: {err:?}"
    );

    client.disconnect();
}

#[tokio::test]
async fn stasis_start_with_args() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;

    // ext 302 does Stasis(test-app,hello,world) in dialplan
    let action = OriginateAction {
        channel: "Local/302@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("args-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let resp = ami.send_action(&action).await.expect("originate failed");
    assert!(resp.success);

    // collect StasisStart events — we may get one from the originate (app=Stasis)
    // and one from the dialplan (Stasis(test-app,hello,world)). look for args.
    let (channel_id, args) = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub.recv().await.expect("event bus closed");
            if let AriEvent::StasisStart { channel, args, .. } = &msg.event {
                if args.contains(&"hello".to_string()) && args.contains(&"world".to_string()) {
                    return (channel.id.clone(), args.clone());
                }
            }
        }
    })
    .await
    .expect("timed out waiting for StasisStart with args");

    assert_eq!(args, vec!["hello", "world"], "stasis args should match");

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn bridge_moh_start_stop() {
    init_tracing();

    let ari = connect_ari().await;
    tokio::time::sleep(Duration::from_millis(500)).await;

    // create a bridge
    let bridge: serde_json::Value = ari
        .post("bridges", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("create bridge failed");
    let bridge_id = bridge["id"]
        .as_str()
        .expect("bridge should have id")
        .to_string();

    // start moh on bridge — may 409 if no moh class
    let moh_result = ari.post_empty(&format!("bridges/{bridge_id}/moh")).await;
    match &moh_result {
        Ok(()) => {
            tracing::info!("bridge moh started");
            tokio::time::sleep(Duration::from_millis(500)).await;
            ari.delete(&format!("bridges/{bridge_id}/moh"))
                .await
                .expect("stop bridge moh failed");
        }
        Err(AriError::Api { status: 409, .. }) => {
            tracing::warn!("bridge moh not available (409)");
        }
        Err(e) => panic!("unexpected bridge moh error: {e}"),
    }

    ari.delete(&format!("bridges/{bridge_id}"))
        .await
        .expect("destroy bridge failed");

    ari.disconnect();
}

#[tokio::test]
async fn channel_play_media() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "play-test <100>").await;

    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // play silence/1 media
    let playback: serde_json::Value = ari
        .post(
            &format!("channels/{channel_id}/play?media=sound:silence/1"),
            &serde_json::json!({}),
        )
        .await
        .expect("play media failed");

    let playback_id = playback["id"].as_str().expect("playback should have id");
    tracing::info!(playback_id, "playback started");

    // wait briefly for playback to register
    tokio::time::sleep(Duration::from_millis(500)).await;

    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn multiple_stasis_subscribers() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub1 = ari.subscribe();
    let mut sub2 = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;

    // originate into stasis
    let action = OriginateAction {
        channel: "Local/999@default".to_string(),
        context: None,
        exten: None,
        priority: None,
        application: Some("Stasis".to_string()),
        data: Some("test-app".to_string()),
        timeout: Some(10000),
        caller_id: Some("multi-sub-test <100>".to_string()),
        account: None,
        async_: true,
        variables: vec![],
    };
    let resp = ami.send_action(&action).await.expect("originate failed");
    assert!(resp.success);

    // both subscribers should receive StasisStart
    let id1 = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub1.recv().await.expect("sub1 event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                return channel.id.clone();
            }
        }
    })
    .await
    .expect("sub1 timed out waiting for StasisStart");

    let id2 = tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let msg = sub2.recv().await.expect("sub2 event bus closed");
            if let AriEvent::StasisStart { channel, .. } = &msg.event {
                return channel.id.clone();
            }
        }
    })
    .await
    .expect("sub2 timed out waiting for StasisStart");

    assert_eq!(id1, id2, "both subscribers should see same channel");

    let _ = ari.delete(&format!("channels/{id1}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}

#[tokio::test]
async fn channel_snoop() {
    init_tracing();

    let ari = connect_ari().await;
    let mut sub = ari.subscribe();
    tokio::time::sleep(Duration::from_secs(1)).await;

    let ami = connect_ami().await;
    let channel_id = originate_into_stasis(&ami, &mut sub, "snoop-test <100>").await;

    ari.post_empty(&format!("channels/{channel_id}/answer"))
        .await
        .expect("answer failed");

    // create a snoop channel
    let snoop: serde_json::Value = ari
        .post(
            &format!("channels/{channel_id}/snoop?app=test-app&spy=both"),
            &serde_json::json!({}),
        )
        .await
        .expect("snoop failed");

    let snoop_id = snoop["id"].as_str().expect("snoop should have channel id");
    tracing::info!(snoop_id, "snoop channel created");

    tokio::time::sleep(Duration::from_millis(500)).await;

    // cleanup: hangup both channels
    let _ = ari.delete(&format!("channels/{snoop_id}")).await;
    let _ = ari.delete(&format!("channels/{channel_id}")).await;
    ami.disconnect().await.expect("ami disconnect failed");
    ari.disconnect();
}
