# Connection & Authentication

## Builder Pattern

`AmiClient` uses a builder to configure and establish connections. The builder
validates configuration and performs the login handshake before returning a
connected client.

```rust,no_run
use asterisk_ami::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("pbx.example.com")
        .port(5038)
        .credentials("admin", "secret")
        .timeout(Duration::from_secs(10))
        .reconnect(ReconnectPolicy::exponential(
            Duration::from_secs(1),
            Duration::from_secs(60),
        ).with_max_retries(5))
        .event_capacity(1024)
        .build()
        .await?;

    Ok(())
}
```

## Builder Methods

| Method | Default | Description |
|--------|---------|-------------|
| `host(addr)` | `"127.0.0.1"` | AMI server hostname or IP |
| `port(n)` | `5038` | TCP port |
| `credentials(user, pass)` | required | Manager username and secret |
| `timeout(duration)` | 30s | Connection and action timeout |
| `reconnect(policy)` | exponential | Reconnection strategy |
| `event_capacity(n)` | 256 | Internal event channel buffer size |

## Authentication

The client supports two authentication methods:

1. **Plaintext** -- sends username and secret directly via the `Login` action.
2. **MD5 challenge-response** -- requests a challenge from Asterisk, computes
   an MD5 digest of the challenge combined with the secret, and sends the
   digest instead of the plaintext secret.

The client selects MD5 automatically when Asterisk supports it. No
configuration is needed.

## Reconnection

When the connection drops, the client reconnects using the configured
`ReconnectPolicy`. The default policy uses exponential backoff with jitter:

- **Initial delay**: 1 second
- **Maximum delay**: 60 seconds
- **Backoff factor**: 2.0x per attempt
- **Jitter**: enabled (randomizes delay to avoid thundering herd)
- **Max retries**: unlimited by default

You can also use a fixed-interval policy or disable reconnection entirely:

```rust,no_run
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

// fixed 5-second retry
let fixed = ReconnectPolicy::fixed(Duration::from_secs(5))
    .with_max_retries(10);

// no reconnection
let none = ReconnectPolicy::none();
```

## Connection State

The client tracks its connection state, which can be one of:

- `Disconnected` -- not connected
- `Connecting` -- initial connection in progress
- `Connected` -- authenticated and operational
- `Reconnecting` -- connection lost, attempting to re-establish

Query the current state with `client.connection_state()`.

## Disconnecting

Call `client.disconnect()` to gracefully close the connection. This sends a
`Logoff` action to Asterisk before closing the TCP socket.
