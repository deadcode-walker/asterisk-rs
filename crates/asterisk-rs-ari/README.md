# asterisk-rs-ari

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ari)](https://crates.io/crates/asterisk-rs-ari)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ari)](https://docs.rs/asterisk-rs-ari)

Async Rust client for the Asterisk REST Interface (ARI). Build Stasis
applications with full call control over REST + WebSocket.

## Example

```rust,ignore
use asterisk_rs_ari::{AriClient, AriConfig};
use asterisk_rs_ari::event::AriEvent;
use asterisk_rs_ari::resources::channel::ChannelHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfig::builder("my-app")
        .host("10.0.0.1")
        .username("asterisk")
        .password("secret")
        .build()?;

    let client = AriClient::connect(config).await?;
    let mut events = client.subscribe();

    while let Some(msg) = events.recv().await {
        if let AriEvent::StasisStart { channel, .. } = msg.event {
            let handle = ChannelHandle::new(channel.id, client.clone());
            handle.answer().await?;
            handle.play("sound:hello-world").await?;
        }
    }

    Ok(())
}
```

## Capabilities

- REST client and WebSocket listener covering the full Asterisk 23 ARI surface
- Typed events with metadata (application, timestamp, asterisk_id) on every event
- Filtered subscriptions -- receive only events you care about
- Resource handles for channels, bridges, playbacks, recordings
- System management -- modules, logging, config, global variables
- URL-safe query encoding for user-provided values
- HTTP connect and request timeouts
- Automatic WebSocket reconnection
- `#[non_exhaustive]` enums -- new variants won't break your code

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-ari)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
