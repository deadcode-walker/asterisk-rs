//! Example: ARI channel recording and playback.
//!
//! Connects to ARI, waits for a channel to enter the Stasis app, records it,
//! then plays the recording back before hanging up.
//!
//! Usage: cargo run --example ari_recording

use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::{AriEvent, AriMessage};
use asterisk_rs_ari::resources::channel::ChannelHandle;
use asterisk_rs_ari::AriClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("recording-demo")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .build()?;

    let client = AriClient::connect(config).await?;
    tracing::info!("connected to ARI");

    let mut events = client.subscribe();

    while let Some(msg) = events.recv().await {
        match msg.event {
            AriEvent::StasisStart { channel, .. } => {
                tracing::info!(channel_id = %channel.id, "stasis start");

                let handle = ChannelHandle::new(channel.id.clone(), client.clone());
                // subscribe before spawning so recording events aren't missed
                // between spawn and the first recv inside the task
                let chan_events = client.subscribe_filtered({
                    let id = channel.id.clone();
                    move |m: &AriMessage| match &m.event {
                        AriEvent::RecordingStarted { recording } => {
                            recording.target_uri.contains(&*id)
                        }
                        AriEvent::RecordingFinished { recording } => {
                            recording.target_uri.contains(&*id)
                        }
                        _ => false,
                    }
                });

                tokio::spawn(async move {
                    if let Err(e) = handle_call(handle, chan_events).await {
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

async fn handle_call(
    channel: ChannelHandle,
    mut events: asterisk_rs_core::event::FilteredSubscription<AriMessage>,
) -> asterisk_rs_ari::error::Result<()> {
    channel.answer().await?;
    tracing::info!(channel_id = %channel.id(), "answered");

    // name must be unique per call — reusing a name requires if_exists="overwrite"
    // in the request body, which the current record() helper does not expose;
    // embedding the channel id makes it naturally unique
    let recording_name = format!("recording-{}", channel.id());
    let _live = channel.record(&recording_name, "wav").await?;
    tracing::info!(%recording_name, "recording initiated");

    // wait for Asterisk to confirm the recording has started
    while let Some(msg) = events.recv().await {
        if let AriEvent::RecordingStarted { recording } = msg.event {
            tracing::info!(
                name = %recording.name,
                state = %recording.state,
                "recording started"
            );
            break;
        }
    }

    // RecordingFinished arrives when max_duration elapses, silence is detected,
    // or the caller hangs up; a real app would pass terminate_on / max_silence
    // as additional JSON fields to the record request
    while let Some(msg) = events.recv().await {
        if let AriEvent::RecordingFinished { recording } = msg.event {
            tracing::info!(
                name = %recording.name,
                format = %recording.format,
                state = %recording.state,
                "recording finished"
            );
            break;
        }
    }

    // play back the recorded audio using the "recording:" URI scheme
    channel.play(&format!("recording:{recording_name}")).await?;
    tracing::info!(%recording_name, "playback started");

    channel.hangup(None).await?;
    tracing::info!(channel_id = %channel.id(), "channel hung up");

    Ok(())
}
