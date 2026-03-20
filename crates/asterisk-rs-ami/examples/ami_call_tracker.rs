//! Example: track completed calls via AMI.
//!
//! Connects to Asterisk AMI, starts a CallTracker, and logs each
//! CompletedCall record (channel, unique_id, duration, cause) as calls
//! finish. A separate task handles incoming records so the main thread
//! remains free for other work.
//!
//! Usage: cargo run --example ami_call_tracker

use asterisk_rs_ami::AmiClient;
use std::time::Duration;
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .ping_interval(Duration::from_secs(20))
        .build()
        .await?;

    info!("connected, starting call tracker");

    // call_tracker() subscribes internally — no events are missed between
    // subscription and tracker start
    let (tracker, mut rx) = client.call_tracker();

    // process completed calls in a background task so the main thread can
    // do other work (originate calls, run dialplan checks, etc.)
    let collector = tokio::spawn(async move {
        while let Some(call) = rx.recv().await {
            info!(
                channel = %call.channel,
                unique_id = %call.unique_id,
                linked_id = %call.linked_id,
                duration_ms = call.duration.as_millis(),
                cause = call.cause,
                cause_txt = %call.cause_txt,
                events = call.events.len(),
                "call completed",
            );
        }
        info!("call tracker receiver closed");
    });

    // wait for ctrl-c, then shut down cleanly
    tokio::signal::ctrl_c().await?;
    info!("shutting down");

    // shutdown signals the background task inside CallTracker to stop,
    // which closes the sender side of the channel, unblocking the collector
    tracker.shutdown();
    collector.await?;

    client.disconnect().await?;
    Ok(())
}
