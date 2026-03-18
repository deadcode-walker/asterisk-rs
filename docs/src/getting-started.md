# Getting Started

Add asterisk-rs to your project:

```toml
[dependencies]
asterisk-rs = "0.2"
```

Or pick individual protocols:

```toml
[dependencies]
asterisk-rs = { version = "0.2", default-features = false, features = ["ami"] }
```

Or use crates directly:

```toml
[dependencies]
asterisk-rs-ami = "0.2"
```

## Protocols

| Protocol | Port | Transport | Crate |
|----------|------|-----------|-------|
| AMI | 5038 | TCP | `asterisk-rs-ami` |
| AGI | 4573 | TCP (FastAGI) | `asterisk-rs-agi` |
| ARI | 8088 | HTTP + WebSocket | `asterisk-rs-ari` |

## Domain Types

Common Asterisk constants are available as typed enums in `asterisk_rs_core::types`:
hangup causes, channel states, device states, dial statuses, and more.
See [Domain Types](./types.md) for the full list.

## Requirements

- Rust 1.83+ (for async fn in trait / RPITIT)
- tokio runtime
- A running Asterisk instance for integration
