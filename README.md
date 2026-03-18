# asterisk-rs

![crates.io](https://img.shields.io/crates/v/asterisk-rs.svg)
![docs.rs](https://img.shields.io/docsrs/asterisk-rs)
![CI](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml/badge.svg)
![MSRV](https://img.shields.io/badge/MSRV-1.83-blue)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)

Async Rust clients for Asterisk AMI, AGI, and ARI.

## Overview

asterisk-rs is a Rust workspace providing typed, async clients for the three
Asterisk integration interfaces. Each protocol lives in its own crate with
shared types and error handling in `asterisk-rs-core`.

## Crates

| Crate | Description |
|---|---|
| `asterisk-rs` | Umbrella crate, re-exports all protocols under feature flags |
| `asterisk-rs-core` | Shared error types, event bus, reconnection policy |
| `asterisk-rs-ami` | AMI client: typed actions, events, codec, reconnection |
| `asterisk-rs-agi` | FastAGI server: handler trait, typed commands |
| `asterisk-rs-ari` | ARI client: REST + WebSocket, typed events, resource handles |

## Quick Start

```sh
cargo add asterisk-rs
```

```rust,no_run
use asterisk_rs::ami::AmiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .build()
        .await?;

    let response = client.ping().await?;
    println!("{response:?}");

    client.disconnect().await?;
    Ok(())
}
```

## Features

- Async/await with tokio
- Typed actions, commands, and events for all three protocols
- Automatic reconnection with exponential backoff and jitter
- MD5 challenge-response authentication (AMI)
- Handle pattern for ARI resources (ChannelHandle, BridgeHandle, etc.)
- Event bus with typed pub/sub
- Structured logging via tracing
- Feature flags for granular dependency control

## Protocols

| Protocol | Port | Transport | Crate |
|---|---|---|---|
| AMI | 5038 | TCP | `asterisk-rs-ami` |
| AGI | 4573 | TCP (FastAGI) | `asterisk-rs-agi` |
| ARI | 8088 | HTTP + WebSocket | `asterisk-rs-ari` |

## MSRV

1.83

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
