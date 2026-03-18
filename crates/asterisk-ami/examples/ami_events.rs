//! Example: subscribe to AMI events.
//!
//! Connects to Asterisk AMI and prints all incoming events.
//!
//! Usage: cargo run --example ami_events

use asterisk_ami::AmiClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .build()
        .await?;

    println!("connected, listening for events...");

    let mut subscription = client.subscribe();

    while let Some(event) = subscription.recv().await {
        println!(
            "[{}] channel={:?} unique_id={:?}",
            event.event_name(),
            event.channel(),
            event.unique_id(),
        );
    }

    Ok(())
}
