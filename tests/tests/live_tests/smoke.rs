use std::time::Duration;

use asterisk_rs_ami::client::AmiClient;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::resources::channel::{self, ExternalMediaParams};
use asterisk_rs_ari::resources::{device_state, mailbox};
use asterisk_rs_ari::{AriClient, TransportMode};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_tests::helpers::{init_tracing, live_config};
use futures_util::{SinkExt, StreamExt};
use tokio::net::TcpListener;
use tokio_tungstenite::tungstenite::Message;
use tokio_tungstenite::tungstenite::handshake::server::{Request, Response};
use tokio_tungstenite::tungstenite::http::{HeaderValue, header::SEC_WEBSOCKET_PROTOCOL};

async fn ami_client() -> AmiClient {
    let config = live_config();
    AmiClient::builder()
        .host(&config.ami_host)
        .port(config.ami_port)
        .credentials(&config.ami_username, &config.ami_secret)
        .reconnect(ReconnectPolicy::none())
        .timeout(Duration::from_secs(10))
        .build()
        .await
        .expect("failed to connect to the selected AMI endpoint")
}

async fn ari_client(mode: TransportMode) -> AriClient {
    let config = live_config();
    let ari = AriConfigBuilder::new(&config.ari_app)
        .host(&config.ari_host)
        .port(config.ari_port)
        .username(&config.ari_username)
        .password(&config.ari_password)
        .transport(mode)
        .reconnect(ReconnectPolicy::exponential(
            Duration::from_millis(250),
            Duration::from_secs(2),
        ))
        .build()
        .expect("failed to build explicit live ARI config");
    AriClient::connect(ari)
        .await
        .expect("failed to connect to the selected ARI endpoint")
}

#[tokio::test]
#[ignore = "requires an explicitly selected owned Asterisk test instance"]
async fn owned_instance_marker_and_ami_ping() {
    init_tracing();
    let config = live_config();
    let ami = ami_client().await;
    let response = ami.ping().await.expect("AMI ping failed");
    assert!(response.success, "AMI ping should succeed: {response:?}");

    let ari = ari_client(TransportMode::Http).await;
    let marker: serde_json::Value = ari
        .get("asterisk/variable?variable=ASTERISK_RS_TEST_INSTANCE")
        .await
        .expect("test-instance marker is not readable through ARI");
    assert_eq!(
        marker.get("value").and_then(serde_json::Value::as_str),
        Some(config.instance_marker.as_str()),
        "selected PBX marker does not match ASTERISK_TEST_INSTANCE_MARKER"
    );

    ami.disconnect().await.expect("AMI disconnect failed");
    ari.disconnect();
}

#[tokio::test]
#[ignore = "requires an explicitly selected owned Asterisk test instance"]
async fn ari_http_and_unified_websocket_get() {
    init_tracing();
    for mode in [TransportMode::Http, TransportMode::WebSocket] {
        let ari = ari_client(mode).await;
        let info: serde_json::Value = ari
            .get("asterisk/info")
            .await
            .unwrap_or_else(|error| panic!("{mode:?} GET asterisk/info failed: {error}"));
        assert!(
            info.get("system").is_some(),
            "{mode:?} response lacks system"
        );
        ari.disconnect();
    }
}

#[tokio::test]
#[ignore = "requires an explicitly selected owned Asterisk test instance"]
async fn device_and_mailbox_put_round_trip() {
    init_tracing();
    let config = live_config();
    let ari = ari_client(TransportMode::Http).await;
    let device = format!("Stasis:{}", config.resource_name("device"));
    let mailbox_name = format!("{}@default", config.resource_name("mailbox"));

    let _ = device_state::delete(&ari, &device).await;
    device_state::update(&ari, &device, "INUSE")
        .await
        .expect("device-state PUT failed");
    let state = device_state::get(&ari, &device).await;
    device_state::delete(&ari, &device)
        .await
        .expect("device-state cleanup failed");
    let state = state.expect("device-state GET after PUT failed");
    assert_eq!(state.name, device);
    assert_eq!(state.state, "INUSE");

    let _ = mailbox::delete(&ari, &mailbox_name).await;
    mailbox::update(&ari, &mailbox_name, 3, 7)
        .await
        .expect("mailbox PUT failed");
    let mailbox = mailbox::get(&ari, &mailbox_name).await;
    mailbox::delete(&ari, &mailbox_name)
        .await
        .expect("mailbox cleanup failed");
    let mailbox = mailbox.expect("mailbox GET after PUT failed");
    assert_eq!(mailbox.name, mailbox_name);
    assert_eq!((mailbox.old_messages, mailbox.new_messages), (3, 7));
    ari.disconnect();
}

#[allow(clippy::result_large_err)] // tungstenite's handshake callback owns the response error type
async fn capture_media_start(port: u16, hangup: Message) -> Message {
    let listener = TcpListener::bind(("0.0.0.0", port))
        .await
        .unwrap_or_else(|error| panic!("failed to bind media fixture port {port}: {error}"));
    let (stream, _) = tokio::time::timeout(Duration::from_secs(10), listener.accept())
        .await
        .expect("Asterisk did not connect to the media WebSocket fixture")
        .expect("media fixture accept failed");
    let mut websocket =
        tokio_tungstenite::accept_hdr_async(stream, |request: &Request, mut response: Response| {
            assert_eq!(
                request
                    .headers()
                    .get(SEC_WEBSOCKET_PROTOCOL)
                    .and_then(|value| value.to_str().ok()),
                Some("media"),
                "Asterisk must request the media WebSocket subprotocol"
            );
            response
                .headers_mut()
                .insert(SEC_WEBSOCKET_PROTOCOL, HeaderValue::from_static("media"));
            Ok(response)
        })
        .await
        .expect("Asterisk media WebSocket handshake failed");
    let start = tokio::time::timeout(Duration::from_secs(5), websocket.next())
        .await
        .expect("Asterisk sent no media control frame")
        .expect("Asterisk closed media WebSocket before MEDIA_START")
        .expect("invalid Asterisk media WebSocket frame");
    websocket
        .send(hangup)
        .await
        .expect("failed to send deterministic media HANGUP");
    start
}

#[tokio::test]
#[ignore = "requires the repository Asterisk 22.9+ chan_websocket fixture"]
async fn chan_websocket_plaintext_and_json_media_start_schemas() {
    init_tracing();
    let config = live_config();
    let ari = ari_client(TransportMode::Http).await;

    let plain_id = config.resource_name("media-plain");
    let plain_params = ExternalMediaParams::new(&config.ari_app, "media_plain", "ulaw")
        .encapsulation("none")
        .transport("websocket")
        .channel_id(&plain_id);
    let plain_capture = capture_media_start(8787, Message::Text("HANGUP".into()));
    let (plain_start, plain_channel) =
        tokio::join!(plain_capture, channel::external_media(&ari, &plain_params));
    let plain_channel = plain_channel.expect("plaintext external-media creation failed");
    let plain = plain_start
        .into_text()
        .expect("plaintext MEDIA_START must be a text frame");
    assert!(
        plain.starts_with("MEDIA_START "),
        "unexpected plaintext event: {plain}"
    );
    assert!(
        plain.contains(&format!("channel_id:{plain_id}")),
        "plaintext MEDIA_START lacks isolated channel id: {plain}"
    );
    assert!(
        plain.contains("format:ulaw"),
        "plaintext MEDIA_START lacks format: {plain}"
    );
    assert!(
        plain.contains("optimal_frame_size:160"),
        "plaintext MEDIA_START lacks exact ulaw frame size: {plain}"
    );
    assert_eq!(plain_channel.id, plain_id);

    let json_id = config.resource_name("media-json");
    let json_params = ExternalMediaParams::websocket_json(&config.ari_app, "media_json", "ulaw")
        .channel_id(&json_id);
    let json_capture = capture_media_start(
        8788,
        Message::Text(serde_json::json!({"command": "HANGUP"}).to_string().into()),
    );
    let (json_start, json_channel) =
        tokio::join!(json_capture, channel::external_media(&ari, &json_params));
    let json_channel = json_channel.expect("JSON external-media creation failed");
    let json_text = json_start
        .into_text()
        .expect("JSON MEDIA_START must be a text frame");
    let json: serde_json::Value =
        serde_json::from_str(&json_text).expect("MEDIA_START was not valid JSON");
    assert_eq!(
        json.get("event").and_then(serde_json::Value::as_str),
        Some("MEDIA_START")
    );
    assert_eq!(
        json.get("channel_id").and_then(serde_json::Value::as_str),
        Some(json_id.as_str())
    );
    assert_eq!(
        json.get("format").and_then(serde_json::Value::as_str),
        Some("ulaw")
    );
    assert_eq!(
        json.get("optimal_frame_size")
            .and_then(serde_json::Value::as_u64),
        Some(160)
    );
    assert_eq!(
        json.get("ptime").and_then(serde_json::Value::as_u64),
        Some(20)
    );
    assert_eq!(json_channel.id, json_id);
    ari.disconnect();
}
