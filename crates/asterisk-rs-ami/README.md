# asterisk-rs-ami

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ami)](https://crates.io/crates/asterisk-rs-ami)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ami)](https://docs.rs/asterisk-rs-ami)

Async Rust client for the Asterisk Manager Interface (AMI). Monitor calls,
originate channels, manage queues, and react to real-time events over TCP.

## Example

```rust,ignore
use asterisk_rs_ami::{AmiClient, AmiEvent};
use asterisk_rs_ami::action::StatusAction;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("10.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    // collect all active channel statuses in one call
    let result = client.send_collecting(&StatusAction { channel: None }).await?;
    for event in &result.events {
        println!("{}: {:?}", event.event_name(), event.channel());
    }

    // subscribe to hangup events only
    let mut hangups = client.subscribe_filtered(|e| e.event_name() == "Hangup");
    while let Some(event) = hangups.recv().await {
        if let AmiEvent::Hangup { channel, cause_txt, .. } = event {
            println!("{channel} hung up: {cause_txt}");
        }
    }

    Ok(())
}
```

## Capabilities

- Typed events and actions covering the full Asterisk 23 AMI surface
- Filtered subscriptions -- receive only events you care about
- Event-collecting actions -- `send_collecting()` gathers multi-event responses
- MD5 challenge-response and plaintext authentication
- Automatic reconnection with re-authentication on every reconnect
- Command output capture for `Response: Follows` responses
- Domain types for hangup causes, channel states, device states, and more
- `#[non_exhaustive]` enums -- new variants won't break your code
- Configurable timeouts, backoff, and event buffer size

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-ami)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
