//! Example: ARI with WebSocket transport mode.
//!
//! By default the ARI client uses two separate connections: HTTP for REST calls
//! and a WebSocket for events. WebSocket transport mode collapses both onto a
//! single persistent WebSocket connection — REST requests are sent as frames
//! over that same socket and responses are correlated back by request ID.
//!
//! When to use it:
//! - Asterisk 20.14.0+ / 21.9.0+ / 22.4.0+ (earlier releases lack the
//!   required protocol support)
//! - Environments where opening two outbound connections to Asterisk is
//!   inconvenient (strict firewall rules, NAT traversal, load balancers that
//!   do not preserve HTTP session affinity)
//!
//! The application code is identical to HTTP mode — the transport difference
//! is entirely in the config builder. This example lists active channels, pings
//! Asterisk, then enters an event loop handling StasisStart/StasisEnd.
//!
//! Usage: cargo run --example ari_websocket_transport

use asterisk_rs_ari::config::{AriConfigBuilder, TransportMode};
use asterisk_rs_ari::event::{AriEvent, Channel};
use asterisk_rs_ari::resources::asterisk::{self, AsteriskPing};
use asterisk_rs_ari::AriClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // single WebSocket carries both REST and event traffic
    let config = AriConfigBuilder::new("ws-demo")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .transport(TransportMode::WebSocket)
        .build()?;

    let client = AriClient::connect(config).await?;
    tracing::info!("connected via WebSocket transport");

    // REST calls go over the same WebSocket — no second HTTP connection
    let ping: AsteriskPing = asterisk::ping(&client).await?;
    tracing::info!(
        asterisk_id = %ping.asterisk_id,
        timestamp = %ping.timestamp,
        "asterisk ping ok"
    );

    let channels: Vec<Channel> = client.get("/channels").await?;
    tracing::info!(count = channels.len(), "active channels");
    for ch in &channels {
        tracing::info!(
            id = %ch.id,
            name = %ch.name,
            state = %ch.state,
            "channel"
        );
    }

    // event subscription works identically to HTTP mode
    let mut events = client.subscribe();
    tracing::info!("entering event loop — ctrl-c to stop");

    loop {
        tokio::select! {
            Some(msg) = events.recv() => {
                match msg.event {
                    AriEvent::StasisStart { channel, args, .. } => {
                        tracing::info!(
                            channel_id = %channel.id,
                            channel_name = %channel.name,
                            ?args,
                            "stasis start"
                        );
                    }
                    AriEvent::StasisEnd { channel, .. } => {
                        tracing::info!(channel_id = %channel.id, "stasis end");
                    }
                    AriEvent::ChannelStateChange { channel, .. } => {
                        tracing::info!(
                            channel_id = %channel.id,
                            state = %channel.state,
                            "channel state change"
                        );
                    }
                    AriEvent::ChannelDestroyed { channel, cause_txt, .. } => {
                        tracing::info!(
                            channel_id = %channel.id,
                            cause = %cause_txt,
                            "channel destroyed"
                        );
                    }
                    _ => {}
                }
            }
            _ = tokio::signal::ctrl_c() => {
                tracing::info!("shutting down");
                break;
            }
        }
    }

    Ok(())
}
