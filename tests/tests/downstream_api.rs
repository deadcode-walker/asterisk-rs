//! Compile contract for the intended 0.8 downstream construction surface.

use std::collections::HashMap;

use asterisk_rs_ari::AriError;
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::{AriEvent, AriMessage};
use asterisk_rs_ari::media::{MediaCommand, MediaDirection, MediaEvent};
use asterisk_rs_ari::resources::asterisk::ConfigTuple;
use asterisk_rs_ari::resources::channel::{ExternalMediaParams, OriginateParams};
use asterisk_rs_core::event::EventReceive;

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

    let _ = (originate, external, websocket_external, config, tuple);

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
    );
}
