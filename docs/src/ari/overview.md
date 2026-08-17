# ARI (Asterisk REST Interface)

ARI provides full call control through a REST API combined with a WebSocket
event stream for Stasis applications.

Cleartext HTTP/WebSocket is allowed by default only on loopback. Remote
cleartext requires `.allow_insecure_remote(true)`; prefer `.secure(true)`.
Private PKI deployments can add a PEM CA bundle with `.private_ca_pem(...)`,
which augments platform trust for both HTTPS and WSS.

## Quick Start

```rust,ignore
use asterisk_rs_ari::AriClient;
use asterisk_rs_ari::config::AriConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfigBuilder::new("my-app")
        .host("10.0.0.1")
        .secure(true)
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

- REST/WebSocket clients for the modeled ARI surface, with Asterisk 22 as the live-proven boundary
- Typed events with metadata (application, timestamp, asterisk_id)
- Filtered subscriptions -- receive only events you care about
- Resource handles for channels, bridges, playbacks, recordings
- System management -- modules, logging, config, global variables
- URL-safe query encoding, HTTP timeouts, WebSocket lifecycle management

See [Stasis Applications](./stasis.md) for the event model,
[Resources](./resources.md) for the handle pattern, and
[API reference](./reference.md) for links to the canonical rustdoc inventory.
