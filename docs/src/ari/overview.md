# ARI (Asterisk REST Interface)

ARI provides full call control through a REST API combined with a WebSocket
event stream for Stasis applications.

## Quick Start

```rust,ignore
use asterisk_rs_ari::{AriClient, AriConfig};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfig::builder("my-app")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("secret")
        .build()?;

    let client = AriClient::connect(config).await?;

    // subscribe to stasis events
    let mut events = client.subscribe();
    while let Some(msg) = events.recv().await {
        println!("[{}] {:?}", msg.application, msg.event);
    }

    Ok(())
}
```

## Features

- REST client with all 90 ARI endpoints
- WebSocket event listener with reconnection
- All 43 typed events with metadata (application, timestamp, asterisk_id)
- Resource handles (ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle)
- Filtered subscriptions
- URL encoding for user-provided values
- HTTP connect and request timeouts

See [Stasis Applications](./stasis.md) for the event model,
[Resources](./resources.md) for the handle pattern, and
[Reference](./reference.md) for complete endpoint lists.
