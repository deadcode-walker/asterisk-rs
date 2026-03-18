# Connection & Authentication

## Builder

```rust,ignore
use asterisk_rs_ami::AmiClient;
use asterisk_rs_core::config::ReconnectPolicy;
use std::time::Duration;

let client = AmiClient::builder()
    .host("10.0.0.1")
    .port(5038)
    .credentials("admin", "secret")
    .timeout(Duration::from_secs(10))
    .reconnect(ReconnectPolicy::exponential(
        Duration::from_secs(1),
        Duration::from_secs(30),
    ))
    .event_capacity(2048)
    .build()
    .await?;
```

## Authentication

The client tries MD5 challenge-response first, falling back to plaintext.
Authentication happens automatically during `build()` and after every reconnect.

## Reconnection

When the TCP connection drops, the background task reconnects with exponential
backoff and re-authenticates before setting the connection state to `Connected`.

Policies:
- `ReconnectPolicy::exponential(initial, max)` — doubling delay with jitter
- `ReconnectPolicy::fixed(interval)` — constant delay
- `ReconnectPolicy::none()` — no retry
- `.with_max_retries(n)` — cap attempts

## Connection State

Monitor connection health:

```rust,ignore
use asterisk_rs_core::config::ConnectionState;

let state = client.connection_state();
match state {
    ConnectionState::Connected => { /* ready */ }
    ConnectionState::Reconnecting => { /* waiting */ }
    _ => { /* down */ }
}
```

## Disconnect

```rust,ignore
client.disconnect().await?;
```

Sends a Logoff action before closing the TCP connection.
