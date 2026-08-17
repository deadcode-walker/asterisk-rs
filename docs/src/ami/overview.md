# AMI (Asterisk Manager Interface)

AMI is a TCP protocol on port 5038 for monitoring and controlling Asterisk.
The client handles authentication, reconnection, and event dispatch automatically.

## Quick Start

```rust,ignore
use asterisk_rs_ami::AmiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("10.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let resp = client.ping().await?;
    println!("{resp:?}");

    client.disconnect().await?;
    Ok(())
}
```

## Capabilities

- Typed actions and events for the modeled AMI surface; Asterisk 22 is the live-proven boundary
- MD5 challenge-response with configurable plaintext fallback; remote transport requires an operator-owned, versioned TLS proxy
- Automatic reconnection with re-authentication
- Filtered subscriptions -- receive only events you care about
- Event-collecting actions -- `send_collecting()` gathers multi-event responses
- Command output capture for `Response: Follows`
- Configurable timeouts, backoff, and event buffer size

See [Connection & Authentication](./connection.md) for setup details,
[Events](./events.md) for the event system, and
[API reference](./reference.md) for links to the canonical rustdoc inventory.
