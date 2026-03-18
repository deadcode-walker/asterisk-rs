# asterisk-rs-ami

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ami)](https://crates.io/crates/asterisk-rs-ami)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ami)](https://docs.rs/asterisk-rs-ami)

Async Rust client for the Asterisk Manager Interface (AMI).

Monitor calls, originate channels, manage queues, and react to real-time
events over TCP. Built on tokio with automatic reconnection and MD5 auth.

```rust,ignore
use asterisk_rs_ami::{AmiClient, AmiEvent};

let client = AmiClient::builder()
    .host("10.0.0.1")
    .credentials("admin", "secret")
    .build()
    .await?;

// collect all active channel statuses in one call
let result = client.send_collecting(
    &asterisk_rs_ami::action::StatusAction { channel: None }
).await?;

for event in &result.events {
    println!("{}", event.event_name());
}
```

## Features

- Typed events and actions covering the full Asterisk 23 AMI surface
- Filtered subscriptions -- receive only events you care about
- Event-collecting actions for multi-event responses (Status, QueueStatus, etc.)
- MD5 challenge-response and plaintext authentication
- Automatic reconnection with re-authentication
- Command output capture for CLI responses
- Configurable timeouts, backoff, and event buffer size

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
