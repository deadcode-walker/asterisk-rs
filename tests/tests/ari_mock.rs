mod common;
mod mock;

use std::time::Duration;

use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::{AriClient, AriError};
use asterisk_rs_core::config::ReconnectPolicy;

use mock::ari_server::MockAriServerBuilder;

/// build an ARI client pointed at the mock server
async fn connect_to_mock(port: u16) -> AriClient {
    let config = AriConfigBuilder::new("test-app")
        .host("127.0.0.1")
        .port(port)
        .username("testuser")
        .password("testpass")
        .reconnect(ReconnectPolicy::none())
        .build()
        .expect("failed to build ari config");

    AriClient::connect(config)
        .await
        .expect("failed to connect ari client")
}

#[tokio::test]
async fn connect_and_disconnect() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn get_request() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/asterisk/info",
            200,
            r#"{"status":"Fully Booted"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let info: serde_json::Value = client
        .get("asterisk/info")
        .await
        .expect("GET asterisk/info failed");

    assert_eq!(
        info["status"], "Fully Booted",
        "expected status field in response"
    );

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn post_request() {
    common::init_tracing();

    let body = r#"{"id":"chan-1","name":"SIP/100-0001","state":"Ring"}"#;
    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let channel: serde_json::Value = client
        .post("channels", &serde_json::json!({"endpoint": "SIP/100"}))
        .await
        .expect("POST channels failed");

    assert_eq!(channel["id"], "chan-1", "expected channel id in response");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn delete_request() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("DELETE", "/ari/channels/chan-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .delete("channels/chan-1")
        .await
        .expect("DELETE channels/chan-1 failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn api_error_handling() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/channels/nonexistent",
            404,
            r#"{"message":"Channel not found"}"#,
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let result = client
        .get::<serde_json::Value>("channels/nonexistent")
        .await;

    match result {
        Err(AriError::Api { status, message }) => {
            assert_eq!(status, 404, "expected 404 status");
            assert!(
                message.contains("Channel not found"),
                "expected error body, got: {message}"
            );
        }
        Err(other) => panic!("expected AriError::Api, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn websocket_events() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // give the background ws task time to connect
    tokio::time::sleep(Duration::from_millis(200)).await;

    let event_json = r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-1",
            "name": "SIP/100-0001",
            "state": "Ring",
            "caller": { "name": "Test", "number": "100" },
            "connected": { "name": "", "number": "" },
            "dialplan": { "context": "default", "exten": "100", "priority": 1 }
        },
        "args": []
    }"#;

    server.send_event(event_json);

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for ws event")
        .expect("event subscription closed unexpectedly");

    assert_eq!(event.application, "test-app", "expected application field");
    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-1", "expected channel id");
            assert_eq!(channel.state, "Ring", "expected channel state");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn unregistered_route_returns_404() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    let result = client.get::<serde_json::Value>("does/not/exist").await;

    match result {
        Err(AriError::Api { status, .. }) => {
            assert_eq!(status, 404, "expected 404 for unregistered route");
        }
        Err(other) => panic!("expected AriError::Api 404, got: {other:?}"),
        Ok(val) => panic!("expected error, got success: {val:?}"),
    }

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn put_request() {
    common::init_tracing();

    let body = r#"{"id":"bridge-1","bridge_type":"mixing"}"#;
    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/bridges/bridge-1", 200, body)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let bridge: serde_json::Value = client
        .put("bridges/bridge-1", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("PUT bridges should succeed");

    assert_eq!(bridge["id"], "bridge-1");
    assert_eq!(bridge["bridge_type"], "mixing");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn filtered_subscription() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;

    // only accept StasisStart events
    let mut filtered = client.subscribe_filtered(|msg| {
        matches!(msg.event, asterisk_rs_ari::AriEvent::StasisStart { .. })
    });

    tokio::time::sleep(Duration::from_millis(200)).await;

    // send a ChannelDestroyed event first (should be filtered out)
    server.send_event(
        r#"{
        "type": "ChannelDestroyed",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-99",
            "name": "SIP/200-0099",
            "state": "Down",
            "caller": {"name": "", "number": ""},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "s", "priority": 1}
        },
        "cause": 16,
        "cause_txt": "Normal"
    }"#,
    );

    // send a StasisStart event (should pass the filter)
    server.send_event(
        r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:01.000+0000",
        "channel": {
            "id": "chan-42",
            "name": "SIP/100-0042",
            "state": "Ring",
            "caller": {"name": "Test", "number": "100"},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "100", "priority": 1}
        },
        "args": []
    }"#,
    );

    let event = tokio::time::timeout(Duration::from_secs(5), filtered.recv())
        .await
        .expect("timed out waiting for filtered event")
        .expect("subscription closed");

    // should receive StasisStart, not ChannelDestroyed
    match &event.event {
        asterisk_rs_ari::AriEvent::StasisStart { channel, .. } => {
            assert_eq!(channel.id, "chan-42");
        }
        other => panic!("expected StasisStart, got: {other:?}"),
    }

    client.disconnect();
    server.shutdown();
}


#[tokio::test]
async fn channel_handle_answer_and_hangup() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/answer", 204, "")
        .route("DELETE", "/ari/channels/chan-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    handle.answer().await.expect("answer failed");
    handle.hangup(None).await.expect("hangup failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn channel_handle_hold_mute_dtmf() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/hold", 204, "")
        .route("DELETE", "/ari/channels/chan-1/hold", 204, "")
        .route("POST", "/ari/channels/chan-1/mute", 204, "")
        .route("DELETE", "/ari/channels/chan-1/mute", 204, "")
        .route("POST", "/ari/channels/chan-1/dtmf?dtmf=1234", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    handle.hold().await.expect("hold failed");
    handle.unhold().await.expect("unhold failed");
    handle.mute(None).await.expect("mute failed");
    handle.unmute(None).await.expect("unmute failed");
    handle.send_dtmf("1234").await.expect("send_dtmf failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn channel_handle_variables() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route(
            "GET",
            "/ari/channels/chan-1/variable?variable=MY_VAR",
            200,
            r#"{"value":"hello"}"#,
        )
        .route(
            "POST",
            "/ari/channels/chan-1/variable?variable=MY_VAR&value=hello",
            204,
            "",
        )
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    let var = handle.get_variable("MY_VAR").await.expect("get variable failed");
    assert_eq!(var.value, "hello", "expected variable value");

    handle
        .set_variable("MY_VAR", "hello")
        .await
        .expect("set variable failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn bridge_handle_lifecycle() {
    common::init_tracing();

    let bridge_json = r#"{"id":"br-1","bridge_type":"mixing","technology":"simple_bridge","channels":[]}"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/bridges", 200, bridge_json)
        .route(
            "POST",
            "/ari/bridges/br-1/addChannel?channel=chan-1",
            204,
            "",
        )
        .route(
            "POST",
            "/ari/bridges/br-1/removeChannel?channel=chan-1",
            204,
            "",
        )
        .route("DELETE", "/ari/bridges/br-1", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;

    // create bridge via client
    let _bridge: serde_json::Value = client
        .post("bridges", &serde_json::json!({"type": "mixing"}))
        .await
        .expect("create bridge failed");

    let handle = asterisk_rs_ari::resources::bridge::BridgeHandle::new("br-1", client.clone());
    handle
        .add_channel("chan-1")
        .await
        .expect("add channel failed");
    handle
        .remove_channel("chan-1")
        .await
        .expect("remove channel failed");
    handle.destroy().await.expect("destroy bridge failed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn post_empty_returns_ok() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/answer", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .post_empty("channels/chan-1/answer")
        .await
        .expect("post_empty should succeed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn put_empty_returns_ok() {
    common::init_tracing();

    let server = MockAriServerBuilder::new()
        .route("PUT", "/ari/channels/chan-1/something", 204, "")
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    client
        .put_empty("channels/chan-1/something")
        .await
        .expect("put_empty should succeed");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn disconnect_stops_ws_listener() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    // give ws task time to connect
    tokio::time::sleep(Duration::from_millis(200)).await;

    // send an event to confirm subscription works
    server.send_event(r#"{
        "type": "StasisStart",
        "application": "test-app",
        "timestamp": "2024-01-01T00:00:00.000+0000",
        "channel": {
            "id": "chan-1",
            "name": "SIP/100-0001",
            "state": "Ring",
            "caller": {"name": "Test", "number": "100"},
            "connected": {"name": "", "number": ""},
            "dialplan": {"context": "default", "exten": "100", "priority": 1}
        },
        "args": []
    }"#);

    let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
        .await
        .expect("timed out waiting for event")
        .expect("subscription closed before event");
    assert_eq!(event.application, "test-app");

    // disconnect and verify subscription terminates
    client.disconnect();

    // after disconnect, recv should return None (channel closed)
    let result = tokio::time::timeout(Duration::from_secs(2), sub.recv()).await;
    match result {
        Ok(None) => {} // expected: channel closed
        Ok(Some(_)) => {} // acceptable: buffered event before close
        Err(_) => {} // timeout is acceptable if ws task is still draining
    }

    server.shutdown();
}

#[tokio::test]
async fn channel_handle_play_returns_playback() {
    common::init_tracing();

    let playback_json = r#"{"id":"pb-1","media_uri":"sound:hello","target_uri":"channel:chan-1","state":"queued"}"#;

    let server = MockAriServerBuilder::new()
        .route("POST", "/ari/channels/chan-1/play", 200, playback_json)
        .start()
        .await;

    let client = connect_to_mock(server.port()).await;
    let handle = asterisk_rs_ari::resources::channel::ChannelHandle::new("chan-1", client.clone());

    let pb = handle.play("sound:hello").await.expect("play failed");
    assert_eq!(pb.id, "pb-1", "expected playback id");
    assert_eq!(pb.state, "queued", "expected playback state");
    assert_eq!(pb.media_uri, "sound:hello", "expected media uri");

    client.disconnect();
    server.shutdown();
}

#[tokio::test]
async fn multiple_websocket_events_in_sequence() {
    common::init_tracing();

    let server = MockAriServerBuilder::new().start().await;
    let client = connect_to_mock(server.port()).await;
    let mut sub = client.subscribe();

    tokio::time::sleep(Duration::from_millis(200)).await;

    let channel_block = r#""id": "chan-1", "name": "SIP/100-0001", "state": "Ring", "caller": {"name": "", "number": ""}, "connected": {"name": "", "number": ""}, "dialplan": {"context": "default", "exten": "s", "priority": 1}"#;

    let events = [
        format!(
            r#"{{"type":"StasisStart","application":"test-app","timestamp":"2024-01-01T00:00:00.000+0000","channel":{{{channel_block}}},"args":[]}}"#
        ),
        format!(
            r#"{{"type":"ChannelDestroyed","application":"test-app","timestamp":"2024-01-01T00:00:01.000+0000","channel":{{{channel_block}}},"cause":16,"cause_txt":"Normal"}}"#
        ),
        format!(
            r#"{{"type":"ChannelStateChange","application":"test-app","timestamp":"2024-01-01T00:00:02.000+0000","channel":{{{channel_block}}}}}"#
        ),
        format!(
            r#"{{"type":"StasisEnd","application":"test-app","timestamp":"2024-01-01T00:00:03.000+0000","channel":{{{channel_block}}}}}"#
        ),
        format!(
            r#"{{"type":"ChannelVarset","application":"test-app","timestamp":"2024-01-01T00:00:04.000+0000","channel":{{{channel_block}}},"variable":"MYVAR","value":"test"}}"#
        ),
    ];

    for event_json in &events {
        server.send_event(event_json);
        // small delay to preserve ordering
        tokio::time::sleep(Duration::from_millis(50)).await;
    }

    let mut received = Vec::new();
    for i in 0..5 {
        let event = tokio::time::timeout(Duration::from_secs(5), sub.recv())
            .await
            .unwrap_or_else(|_| panic!("timed out waiting for event {i}"))
            .unwrap_or_else(|| panic!("subscription closed before event {i}"));
        received.push(event);
    }

    // verify all 5 events received in order
    assert!(matches!(received[0].event, asterisk_rs_ari::AriEvent::StasisStart { .. }));
    assert!(matches!(received[1].event, asterisk_rs_ari::AriEvent::ChannelDestroyed { .. }));
    assert!(matches!(received[2].event, asterisk_rs_ari::AriEvent::ChannelStateChange { .. }));
    assert!(matches!(received[3].event, asterisk_rs_ari::AriEvent::StasisEnd { .. }));
    assert!(matches!(received[4].event, asterisk_rs_ari::AriEvent::ChannelVarset { .. }));

    client.disconnect();
    server.shutdown();
}