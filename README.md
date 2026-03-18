# asterisk-rs

[![crates.io](https://img.shields.io/crates/v/asterisk-rs.svg)](https://crates.io/crates/asterisk-rs)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs)](https://docs.rs/asterisk-rs)
[![CI](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.83-blue)](https://blog.rust-lang.org/2024/12/05/Rust-1.83.0.html)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

Async Rust client for Asterisk PBX. Originate calls, handle events, control
channels, bridges, queues, and recordings across all three Asterisk interfaces.

- **AMI** -- monitor and control Asterisk over TCP. Typed events, actions, automatic reconnection, MD5 auth.
- **AGI** -- run dialplan logic from your Rust service. FastAGI server with all 47 commands.
- **ARI** -- full call control via REST + WebSocket. Resource handles, typed events with metadata.

## Example

```rust,ignore
use asterisk_rs::ami::{AmiClient, AmiEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("10.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    // subscribe to hangup events only
    let mut hangups = client.subscribe_filtered(|e| {
        e.event_name() == "Hangup"
    });

    while let Some(event) = hangups.recv().await {
        if let AmiEvent::Hangup { channel, cause, cause_txt, .. } = event {
            println!("{channel} hung up: {cause} ({cause_txt})");
        }
    }

    Ok(())
}
```

## Install

```toml
[dependencies]
asterisk-rs = "0.2"
```

Or pick individual protocols:

```toml
[dependencies]
asterisk-rs-ami = "0.2"   # AMI only
asterisk-rs-agi = "0.1"   # AGI only
asterisk-rs-ari = "0.2"   # ARI only
```

## Capabilities

- Typed actions, events, and commands for the full Asterisk 23 protocol surface
- Filtered event subscriptions -- receive only what you need
- Event-collecting actions -- `send_collecting()` gathers multi-event responses (Status, QueueStatus, etc.)
- Automatic reconnection with exponential backoff, jitter, and re-authentication
- Resource handles for ARI (ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle)
- Domain types for hangup causes, channel states, device states, dial statuses, and more
- ARI event metadata (application, timestamp, asterisk_id) on every event
- AMI command output capture for `Response: Follows`
- URL-safe query encoding, HTTP timeouts, WebSocket lifecycle management
- `#[non_exhaustive]` enums -- new variants won't break your code
- Structured logging via `tracing`

## Protocols

| Protocol | Default Port | Transport | Use Case |
|----------|-------------|-----------|----------|
| [AMI](https://docs.rs/asterisk-rs-ami) | 5038 | TCP | Monitoring, call control, system management |
| [AGI](https://docs.rs/asterisk-rs-agi) | 4573 | TCP | Dialplan logic, IVR, call routing |
| [ARI](https://docs.rs/asterisk-rs-ari) | 8088 | HTTP + WS | Stasis applications, full media control |

## Documentation

- [API Reference (docs.rs)](https://docs.rs/asterisk-rs)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

## MSRV

1.83 -- required for `async fn` in traits (RPITIT).

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
