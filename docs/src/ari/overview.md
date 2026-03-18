# ARI (Asterisk REST Interface)

ARI provides full call control through a REST API combined with a WebSocket
event stream for Stasis applications.

## Quick Start

```rust,ignore
use asterisk_rs_ari::{AriClient, AriConfig};

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
        println!("[{}] {:?}", msg.application, msg.event);
    }

    Ok(())
}
```

## Capabilities

- REST client and WebSocket listener covering the full Asterisk 23 ARI surface
- Typed events with metadata (application, timestamp, asterisk_id)
- Filtered subscriptions -- receive only events you care about
- Resource handles for channels, bridges, playbacks, recordings
- System management -- modules, logging, config, global variables
- URL-safe query encoding, HTTP timeouts, WebSocket lifecycle management

See [Stasis Applications](./stasis.md) for the event model,
[Resources](./resources.md) for the handle pattern, and
[Reference](./reference.md) for complete endpoint lists.
