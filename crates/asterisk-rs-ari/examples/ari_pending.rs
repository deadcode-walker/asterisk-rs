//! Example: race-free channel origination via PendingChannel.
//!
//! The naive originate-then-subscribe pattern has a race: Asterisk fires
//! StasisStart immediately after the channel is created, potentially before
//! the caller has a chance to subscribe. The PendingChannel factory fixes this
//! by subscribing to channel events *before* sending the originate request.
//! Any events that arrive between the REST call returning and the caller calling
//! `events.recv()` are buffered in the subscription channel — none are lost.
//!
//! Flow:
//!   1. `client.channel()` — allocates a pre-generated channel ID, installs a
//!      filtered subscription keyed on that ID (synchronous, no I/O).
//!   2. `pending.originate(params).await?` — sets `channel_id` on the params,
//!      sends the REST request, and returns `(ChannelHandle, FilteredSubscription)`.
//!   3. The caller drives the channel via the handle and reads events from the
//!      subscription without any risk of missing the first events.
//!
//! Usage: cargo run --example ari_pending

use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::AriEvent;
use asterisk_rs_ari::resources::channel::OriginateParams;
use asterisk_rs_ari::AriClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("pending-demo")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .build()?;

    let client = AriClient::connect(config).await?;
    tracing::info!("connected to ARI");

    // step 1: allocate the pending channel — this installs the event filter
    // synchronously before any network traffic goes out
    let pending = client.channel();
    let channel_id = pending.id().to_owned();
    tracing::info!(%channel_id, "pending channel created, event filter active");

    // step 2: originate — channel_id is set automatically from the pending
    // reservation; the event subscription was already live before this call
    let params = OriginateParams {
        endpoint: "PJSIP/100".into(),
        app: Some("pending-demo".into()),
        caller_id: Some("\"Pending Demo\" <0000000000>".into()),
        timeout: Some(30),
        ..Default::default()
    };

    let (handle, mut events) = pending.originate(params).await?;
    tracing::info!(%channel_id, "originate sent, waiting for events");

    // step 3: drive the call by processing the pre-subscribed event stream.
    // events that arrived between originate completing and this loop starting
    // are already buffered — recv() delivers them in order.
    while let Some(msg) = events.recv().await {
        match msg.event {
            AriEvent::StasisStart { channel, args, .. } => {
                tracing::info!(
                    channel_id = %channel.id,
                    ?args,
                    "stasis start — answering"
                );

                // answer and play a greeting; errors are logged and the loop
                // exits so the example terminates cleanly
                if let Err(e) = handle.answer().await {
                    tracing::error!(error = %e, "answer failed");
                    break;
                }

                if let Err(e) = handle.play("sound:hello-world").await {
                    tracing::error!(error = %e, "play failed");
                }

                if let Err(e) = handle.hangup(None).await {
                    tracing::error!(error = %e, "hangup failed");
                }
            }

            AriEvent::ChannelStateChange { channel } => {
                tracing::info!(
                    channel_id = %channel.id,
                    state = %channel.state,
                    "channel state changed"
                );
            }

            AriEvent::StasisEnd { channel } => {
                tracing::info!(channel_id = %channel.id, "stasis end, exiting");
                // channel left the app — we're done
                break;
            }

            AriEvent::ChannelDestroyed {
                channel, cause_txt, ..
            } => {
                tracing::info!(
                    channel_id = %channel.id,
                    cause = %cause_txt,
                    "channel destroyed"
                );
                break;
            }

            _ => {}
        }
    }

    tracing::info!("example complete");
    Ok(())
}
