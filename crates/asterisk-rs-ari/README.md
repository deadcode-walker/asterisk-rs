# asterisk-rs-ari

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ari)](https://crates.io/crates/asterisk-rs-ari)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ari)](https://docs.rs/asterisk-rs-ari)

Async Rust client for the Asterisk REST Interface (ARI).

Build Stasis applications with full call control. Originate channels, manage
bridges, play media, record calls, and react to events over WebSocket.

```rust,ignore
use asterisk_rs_ari::{AriClient, AriConfig, AriEvent};
use asterisk_rs_ari::resources::channel::ChannelHandle;

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
```

## Features

- REST client covering all Asterisk 23 ARI endpoints
- WebSocket event listener with automatic reconnection
- Typed events with metadata (application, timestamp, asterisk_id)
- Resource handles for channels, bridges, playbacks, recordings
- Filtered subscriptions
- System management (modules, logging, config, variables)
- URL-safe encoding, HTTP timeouts

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
