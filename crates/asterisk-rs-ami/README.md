# asterisk-rs-ami

[![Crates.io](https://img.shields.io/crates/v/asterisk-rs-ami)](https://crates.io/crates/asterisk-rs-ami)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-ami)](https://docs.rs/asterisk-rs-ami)
[![License](https://img.shields.io/crates/l/asterisk-rs-ami)](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT)

Async Rust client for the Asterisk Manager Interface (AMI).

AMI is a TCP protocol (default port 5038) for monitoring and controlling an
Asterisk PBX. This crate provides a typed, async client built on tokio with:

- 116 typed actions covering Asterisk 23
- 161 typed events plus an `Unknown` variant for forward compatibility
- MD5 challenge-response authentication
- Automatic reconnection
- Event bus with pub/sub subscriptions
- tokio-util codec for the AMI wire protocol

## Install

```sh
cargo add asterisk-rs-ami
```

## Quick start

```rust,no_run
use asterisk_rs_ami::AmiClient;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("127.0.0.1")
        .port(5038)
        .credentials("admin", "secret")
        .build()
        .await?;

    let response = client.ping().await?;
    println!("pong: {:?}", response);
    Ok(())
}
```

See also the `ami_originate.rs` and `ami_events.rs` examples.

## Workspace

This crate is part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs).

## MSRV

1.83

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-APACHE) or
[MIT License](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT) at your option.
