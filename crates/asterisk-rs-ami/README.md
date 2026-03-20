# asterisk-rs-ami

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ami)](https://crates.io/crates/asterisk-rs-ami)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ami)](https://docs.rs/asterisk-rs-ami)

Async Rust client for the Asterisk Manager Interface (AMI). Monitor calls,
originate channels, manage queues, and react to real-time events over TCP.

## Quick Start

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

## Call Tracker

`call_tracker()` correlates `DialBegin`, `DialEnd`, `Hangup`, and bridge events
into a single `CompletedCall` record per channel pair. The record includes
start/end timestamps, duration, hangup cause, and the ordered event list.

```rust,ignore
use asterisk_rs_ami::AmiClient;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("10.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let (tracker, mut rx) = client.call_tracker();

    tokio::spawn(async move {
        while let Some(call) = rx.recv().await {
            tracing::info!(
                channel = %call.channel,
                duration = ?call.duration,
                cause = %call.cause_txt,
                "call completed"
            );
        }
    });

    // keep the tracker alive for as long as you need it;
    // dropping it stops event correlation
    tokio::signal::ctrl_c().await?;
    tracker.shutdown();

    Ok(())
}
```

## Builder Options

| Option | Default | Description |
|---|---|---|
| `host(h)` | `"127.0.0.1"` | AMI host |
| `port(p)` | `5038` | AMI TCP port |
| `credentials(u, p)` | required | AMI login |
| `timeout(d)` | 30 s | per-action response timeout |
| `ping_interval(d)` | disabled | keep-alive `Ping` cadence; set to detect dead connections |
| `reconnect(policy)` | exponential backoff | `ReconnectPolicy::exponential(min, max)` or `::none()` |
| `event_capacity(n)` | 1024 | broadcast channel depth; drop events when full if subscribers are slow |

```rust,ignore
use asterisk_rs_ami::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("10.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .ping_interval(Duration::from_secs(20))
        .reconnect(ReconnectPolicy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(60),
        ))
        .event_capacity(2048)
        .build()
        .await?;

    Ok(())
}
```

## Capabilities

- Typed events and actions covering the full Asterisk 23 AMI surface
- Filtered subscriptions -- receive only events you care about
- Event-collecting actions -- `send_collecting()` gathers multi-event responses
- Call tracking with `CallTracker` -- correlates events into `CompletedCall` records
- MD5 challenge-response and plaintext authentication
- Automatic reconnection with re-authentication on every reconnect
- Command output capture for `Response: Follows` responses
- Channel variable extraction -- `ChanVariable(name)` headers parsed into a dedicated map
- Keep-alive pings with configurable interval -- detects dead connections without sending traffic
- Connection state monitoring via `client.connection_state()`
- Domain types for hangup causes, channel states, device states, and more
- `#[non_exhaustive]` enums -- new variants won't break your code
- Configurable timeouts, backoff, ping interval, and event buffer size

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-ami)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
