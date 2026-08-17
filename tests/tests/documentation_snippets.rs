//! Compile and behavior proof for representative public documentation snippets.

use std::time::Duration;

use asterisk_rs_agi::AgiChannel;
use asterisk_rs_ami::action::{AmiAction, OriginateAction};
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::resources::channel::{ExternalMediaParams, OriginateParams};
use asterisk_rs_core::config::ReconnectPolicy;
use asterisk_rs_core::types::{AgiStatus, ChannelState, HangupCause};

const UMBRELLA_RUSTDOC: &str = include_str!("../../crates/asterisk-rs/src/lib.rs");

#[test]
fn readme_builders_and_domain_types_match_the_public_api() {
    assert!(UMBRELLA_RUSTDOC.contains("asterisk-rs = { version = \"0.8\""));
    assert!(UMBRELLA_RUSTDOC.contains("asterisk-rs-ami = \"0.8\""));
    assert!(!UMBRELLA_RUSTDOC.contains("asterisk-ami"));
    assert!(!UMBRELLA_RUSTDOC.contains("version = \"0.1\""));

    let ami = OriginateAction::new("PJSIP/100")
        .context("default")
        .extension("200")
        .priority(1)
        .caller_id("Example <100>");
    assert_eq!(ami.action_name(), "Originate");

    let ari = OriginateParams::new("PJSIP/100")
        .app("example")
        .caller_id("Example <100>");
    let media = ExternalMediaParams::websocket_json("example", "connection-id", "ulaw");
    let config = AriConfigBuilder::new("example")
        .username("example")
        .password("redacted")
        .build()
        .expect("loopback configuration is valid");
    let reconnect =
        ReconnectPolicy::exponential(Duration::from_millis(100), Duration::from_secs(5));

    assert_eq!(
        serde_json::to_value(ari).expect("originate params serialize")["endpoint"],
        "PJSIP/100"
    );
    assert_eq!(
        serde_json::to_value(media).expect("media params serialize")["format"],
        "ulaw"
    );
    assert_eq!(config.app_name(), "example");
    assert!(reconnect.validate().is_ok());
    assert_eq!(
        HangupCause::from_code(16),
        Some(HangupCause::NormalClearing)
    );
    assert_eq!(ChannelState::from_code(1), Some(ChannelState::Reserved));
    assert_eq!(AgiStatus::from_code(510), Some(AgiStatus::InvalidCommand));
}

// The body is not run because it requires a live FastAGI session. Keeping it in an external test
// still makes rustc check the README's exact method names, argument counts, and argument types.
#[allow(dead_code)]
async fn documented_agi_channel_commands(
    channel: &mut AgiChannel,
) -> asterisk_rs_agi::error::Result<()> {
    channel.stream_file("welcome", "#").await?;
    channel
        .control_stream_file("music", "#", Some(3_000), Some("6"), Some("4"), Some("5"))
        .await?;
    channel
        .record_file("message", "wav", "#", 30_000, true, Some(2))
        .await?;
    Ok(())
}
