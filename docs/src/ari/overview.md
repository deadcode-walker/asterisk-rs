# ARI (Asterisk REST Interface)

The Asterisk REST Interface (ARI) exposes Asterisk functionality through a
combination of HTTP REST endpoints and a WebSocket event stream. It is designed
for building custom communications applications using the Stasis framework.

## How ARI Differs from AMI and AGI

| | AMI | AGI | ARI |
|---|-----|-----|-----|
| **Transport** | TCP (text) | TCP (text) | HTTP + WebSocket |
| **Direction** | Bidirectional | Request-response | REST + push events |
| **Scope** | System management | Single call scripting | Application development |
| **Concurrency** | Async events | Synchronous per call | Fully async |

AMI is for monitoring and management. AGI is for scripting individual call
flows. ARI is for building full applications that control channels, bridges,
recordings, and playbacks with fine-grained control.

## Architecture

ARI uses two communication channels:

- **HTTP REST API** -- for control operations: creating channels, joining
  bridges, starting playbacks, managing recordings. The client authenticates
  with HTTP basic auth on each request.
- **WebSocket** -- for receiving real-time events. When a channel enters a
  Stasis application, Asterisk pushes events over the WebSocket describing
  everything that happens to that channel.

## Quick Start

```rust,no_run
use asterisk_ari::{AriClient, AriConfig};
use asterisk_ari::config::AriConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfigBuilder::new()
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("secret")
        .app_name("my-app")
        .build()?;

    let client = AriClient::connect(config).await?;

    let mut sub = client.subscribe();
    while let Ok(event) = sub.recv().await {
        println!("{:?}", event);
    }

    Ok(())
}
```

## Stasis Model

In your Asterisk dialplan, route calls into a Stasis application:

```text
exten => 100,1,Stasis(my-app)
```

Once a channel enters Stasis, Asterisk stops processing dialplan and hands
full control to your ARI application. You receive events and issue REST
commands to control the call.

See [Stasis Applications](./stasis.md) for details on the event loop and
[Resources](./resources.md) for the available control operations.
