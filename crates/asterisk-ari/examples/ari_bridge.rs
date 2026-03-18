//! Example: ARI bridge two channels.
//!
//! Creates a bridge, originates two channels, and adds them to the bridge.
//!
//! Usage: cargo run --example ari_bridge

use asterisk_ari::config::AriConfigBuilder;
use asterisk_ari::resources::bridge::{self, BridgeHandle};
use asterisk_ari::resources::channel::{self, OriginateParams};
use asterisk_ari::AriClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("bridge-demo")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .build()?;

    let client = AriClient::connect(config).await?;
    tracing::info!("connected to ARI");

    // create a mixing bridge
    let bridge_data = bridge::create(&client, Some("mixing"), Some("demo-bridge")).await?;
    let bridge_handle = BridgeHandle::new(bridge_data.id.clone(), client.clone());
    tracing::info!(bridge_id = %bridge_data.id, "bridge created");

    // originate first channel into the stasis app
    let params_a = OriginateParams {
        endpoint: "SIP/100".into(),
        app: Some("bridge-demo".into()),
        ..Default::default()
    };
    let ch_a = channel::originate(&client, &params_a).await?;
    tracing::info!(channel = %ch_a.id, "channel A originated");

    // originate second channel
    let params_b = OriginateParams {
        endpoint: "SIP/200".into(),
        app: Some("bridge-demo".into()),
        ..Default::default()
    };
    let ch_b = channel::originate(&client, &params_b).await?;
    tracing::info!(channel = %ch_b.id, "channel B originated");

    // add both to the bridge
    bridge_handle.add_channel(&ch_a.id).await?;
    bridge_handle.add_channel(&ch_b.id).await?;
    tracing::info!("both channels bridged");

    // wait for user interrupt
    tokio::signal::ctrl_c().await?;

    // cleanup
    bridge_handle.destroy().await?;
    tracing::info!("bridge destroyed");

    Ok(())
}
