# asterisk-rs

Production Rust client for Asterisk AMI, AGI, and ARI.

![crates.io](https://img.shields.io/crates/v/asterisk-rs.svg)
![docs.rs](https://img.shields.io/docsrs/asterisk-rs)
![CI](https://github.com/deadcode-walker/asterisk-rs/actions/workflows/ci.yml/badge.svg)
![MSRV](https://img.shields.io/badge/MSRV-1.75-blue)
![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-blue)

## Overview

asterisk-rs is a Rust workspace providing typed, async clients for the three
Asterisk integration interfaces: AMI (management), AGI (dialplan gateway), and
ARI (REST + WebSocket). Each protocol lives in its own crate with shared types
and error handling in `asterisk-rs-core`.

## Crates

| Crate | Description | Status |
|---|---|---|
| `asterisk-rs-core` | Shared error types, event bus, configuration | In progress |
| `asterisk-ami` | AMI TCP client with MD5 auth and typed events | Planned |
| `asterisk-agi` | FastAGI server for dialplan scripting | Planned |
| `asterisk-ari` | ARI REST + WebSocket client | Planned |
| `asterisk-rs` | Umbrella crate re-exporting all sub-crates | Planned |

## Quick Start

```sh
cargo add asterisk-rs
```

```rust,no_run
use asterisk_rs::ami;

// AMI example — requires running Asterisk
#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut client = ami::Client::connect("127.0.0.1:5038").await?;
    client.login("admin", "secret").await?;

    let response = client.action("CoreShowChannels", &[]).await?;
    println!("{response:?}");

    client.logoff().await?;
    Ok(())
}
```

## Features

- Typed AMI events and actions with serde deserialization
- Automatic reconnection with configurable backoff
- Structured logging via `tracing`
- TLS support through rustls
- AGI handle pattern for stateful dialplan sessions

## MSRV

1.75

## License

Licensed under either of

- Apache License, Version 2.0 ([LICENSE-APACHE](LICENSE-APACHE) or <http://www.apache.org/licenses/LICENSE-2.0>)
- MIT License ([LICENSE-MIT](LICENSE-MIT) or <http://opensource.org/licenses/MIT>)

at your option.

## Contributing

See [CONTRIBUTING.md](CONTRIBUTING.md) for guidelines.
