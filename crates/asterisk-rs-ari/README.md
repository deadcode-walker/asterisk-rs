# asterisk-rs-ari

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ari)](https://crates.io/crates/asterisk-rs-ari)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ari)](https://docs.rs/asterisk-rs-ari)

Async Rust client for the Asterisk REST Interface (ARI). Build Stasis
applications with full call control over REST + WebSocket.

## Quick Start

```rust,ignore
use asterisk_rs_ari::{AriClient};
use asterisk_rs_ari::config::AriConfigBuilder;
use asterisk_rs_ari::event::AriEvent;
use asterisk_rs_ari::resources::channel::ChannelHandle;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = AriConfigBuilder::new("my-app")
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

## Transport Modes

ARI supports two transport modes, selected at config time.

**HTTP** (default) — standard REST + WebSocket for events. Works with all
supported Asterisk versions.

**WebSocket** — REST calls are tunnelled over the same WebSocket connection as
events. Requires Asterisk 22+ with the unified WebSocket ARI transport enabled.

```rust,ignore
use asterisk_rs_ari::config::{AriConfigBuilder, TransportMode};

let config = AriConfigBuilder::new("my-app")
    .host("127.0.0.1")
    .username("asterisk")
    .password("asterisk")
    .transport(TransportMode::WebSocket)
    .build()?;
```

## Race-Free Resource Creation

Subscribing to a channel's events *after* originating it creates a window where
early events (including `StasisStart`) can be missed. `PendingChannel` eliminates
that race by registering the event subscription before issuing the REST call.

```rust,ignore
use asterisk_rs_ari::resources::channel::OriginateParams;

let pending = client.channel();
let params = OriginateParams {
    endpoint: "PJSIP/100".into(),
    app: Some("my-app".into()),
    ..Default::default()
};
let (handle, mut events) = pending.originate(params).await?;
// events are buffered from creation — StasisStart is guaranteed captured

while let Some(msg) = events.recv().await {
    // handle per-channel events
}
```

The same pattern applies to bridges (`client.bridge()`) and playbacks
(`client.playback()`).

## Resource Handles

Handles wrap a resource ID and a cloned client reference, exposing typed
methods without requiring you to construct REST paths manually.

**ChannelHandle**
- `answer()`, `hangup(reason)`
- `play(media_uri)`, `record(name, format, ...)`
- `mute(direction)`, `unmute(direction)`
- `hold()`, `unhold()`
- `send_dtmf(digit, ...)`
- `get_variable(name)`, `set_variable(name, value)`

**BridgeHandle**
- `add_channel(channel_id)`, `remove_channel(channel_id)`
- `play(media_uri)`
- `start_moh(class)`, `stop_moh()`
- `destroy()`

**PlaybackHandle**
- `pause()`, `unpause()`, `restart()`, `stop()`

**RecordingHandle**
- `stop()`, `pause()`, `unpause()`, `mute()`, `unmute()`, `get()`

## Outbound WebSocket Server

For Asterisk 22+ deployments, ARI can connect outbound to your application
instead of the reverse. `AriServer` listens for those incoming Asterisk
connections.

```rust,ignore
use asterisk_rs_ari::server::AriServer;

let (server, shutdown) = AriServer::builder()
    .bind("0.0.0.0:8765")
    .build().await?;

server.run(|session| async move {
    let mut events = session.subscribe();
    while let Some(msg) = events.recv().await {
        tracing::info!(event = ?msg.event, "received");
    }
}).await?;
```

## Media Channel

`MediaChannel` provides raw audio exchange via `chan_websocket`. Use it to
stream audio directly to/from an Asterisk channel without a separate media
server.

```rust,ignore
use asterisk_rs_ari::media::MediaChannel;

let media = MediaChannel::connect("ws://asterisk:8088/ws").await?;
media.answer().await?;

while let Some(audio) = media.recv_audio().await? {
    // process or echo audio bytes
    media.send_audio(audio).await?;
}
```

## Capabilities

- REST client and WebSocket listener covering the full Asterisk 23 ARI surface
- Typed events with metadata (application, timestamp, asterisk_id) on every event
- Filtered subscriptions -- receive only events you care about
- Race-free resource creation with PendingChannel, PendingBridge, PendingPlayback
- Resource handles for channels, bridges, playbacks, recordings
- Dual transport modes: HTTP and unified WebSocket (Asterisk 22+)
- Outbound WebSocket server for Asterisk 22+ outbound connections
- Media channel for raw audio exchange via chan_websocket
- System management -- modules, logging, config, global variables
- URL-safe query encoding for user-provided values
- HTTP connect and request timeouts
- Automatic WebSocket reconnection
- `#[non_exhaustive]` enums -- new variants won't break your code

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-ari)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
