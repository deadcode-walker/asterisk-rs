//! Compile contract for the intended 0.8 downstream construction surface.

use std::collections::HashMap;

use asterisk_rs_ami::action::{
    AgiAction, FaxSessionAction, FaxSessionsAction, FaxStatsAction, PlayDtmfAction,
};
use asterisk_rs_ari::AriError;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::{AriEvent, AriMessage};
use asterisk_rs_ari::media::{MediaCommand, MediaDirection, MediaEvent};
use asterisk_rs_ari::resources::asterisk::ConfigTuple;
use asterisk_rs_ari::resources::bridge::BridgeHandle;
use asterisk_rs_ari::resources::channel::{ChannelHandle, ExternalMediaParams, OriginateParams};
use asterisk_rs_core::event::EventReceive;
use asterisk_rs_core::types::{ChannelState, DeviceState, ExtensionState, HangupCause};

#[test]
fn downstream_uses_owned_builders_and_future_facing_matches() {
    let variables = HashMap::from([("ACCOUNT".to_owned(), "example".to_owned())]);
    let originate = OriginateParams::new("PJSIP/100")
        .app("example")
        .app_args("one,two")
        .caller_id("Example <100>")
        .variables(variables.clone());
    let external = ExternalMediaParams::new("example", "127.0.0.1:9000", "ulaw")
        .transport("udp")
        .variables(variables);
    let websocket_external =
        ExternalMediaParams::websocket_json("example", "connection-id", "ulaw")
            .data("example-args");
    let config = AriConfigBuilder::new("example")
        .username("example")
        .password("redacted")
        .build()
        .expect("fixture config is valid");
    let tuple = ConfigTuple::new("type", "friend");
    let channel_state: ChannelState = "Up".parse().expect("known state");
    let device_state: DeviceState = "INUSE".parse().expect("known state");
    let hangup_cause = HangupCause::try_from(16).expect("known cause");
    let extension_state = ExtensionState::from(9);
    let rust_style_actions = (
        AgiAction {
            channel: "PJSIP/100".into(),
            command: "EXEC Answer".into(),
            command_id: None,
        },
        PlayDtmfAction {
            channel: "PJSIP/100".into(),
            digit: "1".into(),
            duration: None,
        },
        FaxSessionAction {
            session_number: "1".into(),
        },
        FaxSessionsAction,
        FaxStatsAction,
    );

    let _ = (
        originate,
        external,
        websocket_external,
        config,
        tuple,
        channel_state,
        device_state,
        hangup_cause,
        extension_state,
        rust_style_actions,
    );

    #[allow(deprecated)]
    fn legacy_action_aliases_compile() {
        use asterisk_rs_ami::action::{
            AGIAction, FAXSessionAction, FAXSessionsAction, FAXStatsAction, PlayDTMFAction,
        };
        let _ = AGIAction {
            channel: String::new(),
            command: String::new(),
            command_id: None,
        };
        let _ = PlayDTMFAction {
            channel: String::new(),
            digit: String::new(),
            duration: None,
        };
        let _ = FAXSessionAction {
            session_number: String::new(),
        };
        let _ = (FAXSessionsAction, FAXStatsAction);
    }

    async fn accept_lifecycle(
        client: &asterisk_rs_ari::AriClient,
        channel: &ChannelHandle,
        bridge: &BridgeHandle,
    ) {
        let params = OriginateParams::new("PJSIP/100").app("example");
        let external = ExternalMediaParams::new("example", "127.0.0.1:9000", "ulaw");
        let _ = client.pending_channel();
        let _ = client.pending_bridge();
        let _ = client.pending_playback();
        let _ = asterisk_rs_ari::resources::channel::originate_handle(client, &params).await;
        let _ = asterisk_rs_ari::resources::channel::create_handle(client, "PJSIP/100", "example")
            .await;
        let _ = asterisk_rs_ari::resources::channel::external_media_handle(client, &external).await;
        let _ = channel.play_handle("sound:demo-congrats").await;
        let _ = channel.record_handle("example", "wav").await;
        let _ = bridge.play_handle("sound:demo-congrats").await;
        let _ = bridge.record_handle("example", "wav").await;
    }

    #[allow(clippy::single_match)]
    fn accept_event(event: AriEvent) {
        match event {
            AriEvent::Unknown => {}
            _ => {}
        }
    }
    #[allow(clippy::single_match)]
    fn accept_message(message: AriMessage) {
        match message.event {
            AriEvent::Unknown => {}
            _ => {}
        }
    }
    fn accept_media(event: MediaEvent, command: MediaCommand) {
        #[allow(clippy::single_match)]
        match event {
            MediaEvent::MediaXon { .. } => {}
            _ => {}
        }
        #[allow(clippy::single_match)]
        match command {
            MediaCommand::Answer => {}
            _ => {}
        }
        let _ = MediaDirection::Both;
    }
    fn accept_error(error: AriError) {
        let _ = error.to_string();
    }
    fn accept_receive(outcome: EventReceive<AriMessage>) {
        match outcome {
            EventReceive::Event(_) | EventReceive::Lagged(_) | EventReceive::Closed => {}
        }
    }

    let _ = (
        accept_event,
        accept_message,
        accept_media,
        accept_error,
        accept_receive,
        accept_lifecycle,
        legacy_action_aliases_compile,
    );
}
