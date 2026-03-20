# asterisk-rs

[![crates.io](https://img.shields.io/crates/v/asterisk-rs.svg)](https://crates.io/crates/asterisk-rs)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs)](https://docs.rs/asterisk-rs)
[![CI](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml/badge.svg)](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml)
[![MSRV](https://img.shields.io/badge/MSRV-1.83-blue)](https://blog.rust-lang.org/2024/12/05/Rust-1.83.0.html)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)](#license)

Async Rust client for Asterisk PBX. Originate calls, handle events, control
channels, bridges, queues, and recordings across all three Asterisk interfaces.

- **AMI** -- monitor and control Asterisk over TCP. Typed events, actions, automatic reconnection, MD5 auth.
- **AGI** -- run dialplan logic from your Rust service. FastAGI server with typed async commands.
- **ARI** -- full call control via REST + WebSocket. Resource handles, typed events with metadata.

## Quick Example

```rust,ignore
use asterisk_rs::ami::{AmiClient, AmiEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

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
            tracing::info!(%channel, %cause, %cause_txt, "channel hung up");
        }
    }

    Ok(())
}
```

## Install

Use the umbrella crate to pull in whichever protocols you need:

```toml
[dependencies]
asterisk-rs = "0.1"
```

Or add individual protocol crates directly:

```toml
[dependencies]
asterisk-rs-ami = "0.4"   # AMI only
asterisk-rs-agi = "0.2"   # AGI only
asterisk-rs-ari = "0.4"   # ARI only
```

## Feature Selection

The umbrella crate enables all protocols by default. To select only what you need:

```toml
[dependencies]
asterisk-rs = { version = "0.1", default-features = false, features = ["ami"] }
# or: features = ["agi"]
# or: features = ["ari"]
# or: features = ["ami", "ari"]
```

Available features: `ami`, `agi`, `ari`. The `pbx` abstraction requires `ami`.

## Protocols

| Protocol | Default Port | Transport | Use Case |
|----------|-------------|-----------|----------|
| [AMI](https://docs.rs/asterisk-rs-ami) | 5038 | TCP | Monitoring, call control, system management |
| [AGI](https://docs.rs/asterisk-rs-agi) | 4573 | TCP | Dialplan logic, IVR, call routing |
| [ARI](https://docs.rs/asterisk-rs-ari) | 8088 | HTTP + WS | Stasis applications, full media control |

## Capabilities

- Typed actions, events, and commands for the full Asterisk protocol surface
- Filtered event subscriptions -- receive only what you need
- Event-collecting actions -- `send_collecting()` gathers multi-event responses (Status, QueueStatus, etc.)
- Automatic reconnection with exponential backoff, jitter, and re-authentication
- **Call tracker** -- correlates AMI events into `CompletedCall` records (channel, duration, cause, full event log)
- **PBX abstraction** -- `Pbx::dial()` wraps originate + OriginateResponse correlation into one async call
- **Pending resources** -- ARI `PendingChannel`/`PendingBridge` pre-subscribe before REST to eliminate event races
- **Transport modes** -- ARI supports HTTP (request/response) or WebSocket (bidirectional streaming)
- **Outbound WebSocket server** -- `AriServer` accepts Asterisk 22+ outbound WS connections
- **Media channel** -- low-level audio I/O over WebSocket for external media applications
- Resource handles for ARI (ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle)
- Domain types for hangup causes, channel states, device states, dial statuses, and more
- ARI event metadata (application, timestamp, asterisk_id) on every event
- AMI command output capture for `Response: Follows`
- URL-safe query encoding, HTTP timeouts, WebSocket lifecycle management
- `#[non_exhaustive]` enums -- new variants won't break your code
- Structured logging via `tracing`

## More Examples

### AMI: call tracker

```rust,ignore
use asterisk_rs::ami::AmiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let (tracker, mut rx) = client.call_tracker();

    while let Some(call) = rx.recv().await {
        tracing::info!(
            channel = %call.channel,
            duration = ?call.duration,
            cause = %call.cause_txt,
            "call completed"
        );
    }

    tracker.shutdown();
    Ok(())
}
```

### AGI: IVR handler

```rust,ignore
use asterisk_rs::agi::{AgiChannel, AgiHandler, AgiRequest, AgiServer};

struct IvrHandler;

impl AgiHandler for IvrHandler {
    async fn handle(&self, _request: AgiRequest, mut channel: AgiChannel)
        -> asterisk_rs::agi::error::Result<()>
    {
        channel.answer().await?;
        channel.stream_file("welcome", "#").await?;
        let response = channel.get_data("press-ext", 5000, 4).await?;
        tracing::info!(digits = response.result, "caller input");
        channel.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let (server, _shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(IvrHandler)
        .max_connections(100)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

### ARI: pending channel

```rust,ignore
use asterisk_rs::ari::config::AriConfigBuilder;
use asterisk_rs::ari::{AriClient, AriEvent, PendingChannel, TransportMode};
use asterisk_rs::ari::resources::channel::OriginateParams;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("my-app")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("asterisk")
        .build()?;

    let client = AriClient::connect(config).await?;

    // pre-subscribe before originate so no events are missed
    let pending = client.channel();
    let params = OriginateParams {
        endpoint: "PJSIP/100".into(),
        app: Some("my-app".into()),
        ..Default::default()
    };
    let (handle, mut events) = pending.originate(params).await?;

    while let Some(msg) = events.recv().await {
        match msg.event {
            AriEvent::StasisStart { .. } => {
                handle.answer().await?;
                handle.play("sound:hello-world").await?;
                handle.hangup(None).await?;
            }
            AriEvent::ChannelDestroyed { cause_txt, .. } => {
                tracing::info!(%cause_txt, "channel destroyed");
                break;
            }
            _ => {}
        }
    }

    Ok(())
}
```

### PBX: dial and wait

```rust,ignore
use asterisk_rs::ami::AmiClient;
use asterisk_rs::pbx::{DialOptions, Pbx};
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let client = AmiClient::builder()
        .host("127.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let mut pbx = Pbx::new(client);

    let call = pbx.dial(
        "PJSIP/100",
        "200",
        Some(
            DialOptions::new()
                .caller_id("Rust PBX <100>")
                .timeout_ms(30000),
        ),
    ).await?;

    call.wait_for_answer(Duration::from_secs(30)).await?;
    tracing::info!("call answered");

    call.hangup().await?;

    if let Some(completed) = pbx.next_completed_call().await {
        tracing::info!(duration = ?completed.duration, cause = %completed.cause_txt, "call record");
    }

    Ok(())
}
```

## Documentation

- [API Reference (docs.rs)](https://docs.rs/asterisk-rs)
- [AMI crate docs](https://docs.rs/asterisk-rs-ami)
- [AGI crate docs](https://docs.rs/asterisk-rs-agi)
- [ARI crate docs](https://docs.rs/asterisk-rs-ari)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

## MSRV

1.83 -- required for `async fn` in traits (RPITIT).

## License

Licensed under either of

- [Apache License, Version 2.0](LICENSE-APACHE)
- [MIT License](LICENSE-MIT)

at your option.
