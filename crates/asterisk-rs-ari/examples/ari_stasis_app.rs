//! Example: ARI Stasis application.
//!
//! Connects to ARI, subscribes to events, and handles incoming calls
//! by answering and playing a sound file.
//!
//! Usage: cargo run --example ari_stasis_app

use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::AriEvent;
use asterisk_rs_ari::resources::channel::ChannelHandle;
use asterisk_rs_ari::AriClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("hello-world")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .build()?;

    let client = AriClient::connect(config).await?;
    tracing::info!("connected to ARI");

    let mut events = client.subscribe();

    while let Some(event) = events.recv().await {
        match event {
            AriEvent::StasisStart { channel, args, .. } => {
                tracing::info!(
                    channel_id = %channel.id,
                    channel_name = %channel.name,
                    ?args,
                    "stasis start"
                );

                let handle = ChannelHandle::new(channel.id.clone(), client.clone());
                tokio::spawn(async move {
                    if let Err(e) = handle_call(handle).await {
                        tracing::error!(error = %e, "call handling failed");
                    }
                });
            }
            AriEvent::StasisEnd { channel, .. } => {
                tracing::info!(channel_id = %channel.id, "stasis end");
            }
            _ => {}
        }
    }

    Ok(())
}

async fn handle_call(channel: ChannelHandle) -> asterisk_rs_ari::error::Result<()> {
    channel.answer().await?;
    channel.play("sound:hello-world").await?;
    channel.hangup(None).await?;
    Ok(())
}
