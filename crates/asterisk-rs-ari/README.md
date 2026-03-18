# asterisk-rs-ari

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-ari.svg)](https://crates.io/crates/asterisk-rs-ari)
[![docs.rs](https://docs.rs/asterisk-rs-ari/badge.svg)](https://docs.rs/asterisk-rs-ari)
[![license](https://img.shields.io/crates/l/asterisk-rs-ari.svg)](https://github.com/deadcode-walker/asterisk-rs)

Async Rust client for the Asterisk REST Interface (ARI).

ARI exposes full call control through a REST API combined with a WebSocket
event stream for Stasis applications (default port 8088). This crate provides:

- REST client built on reqwest with Basic Auth
- WebSocket event listener with automatic reconnection
- 43 typed events deserialized via serde tagged unions
- Resource handle pattern: `ChannelHandle`, `BridgeHandle`, `PlaybackHandle`, `RecordingHandle`

## Install

```sh
cargo add asterisk-rs-ari
```

## Quick start

```rust,no_run
use asterisk_rs_ari::AriClient;
use asterisk_rs_ari::config::AriConfigBuilder;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = AriConfigBuilder::new("my-app")
        .host("127.0.0.1")
        .port(8088)
        .username("asterisk")
        .password("secret")
        .build()?;

    let client = AriClient::connect(config).await?;
    // subscribe to Stasis events, control channels, bridges, etc.
    Ok(())
}
```

See `examples/ari_stasis_app.rs` and `examples/ari_bridge.rs` for working
examples with event handling and bridge management.

## Part of asterisk-rs

This crate is part of the [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs)
workspace. See the umbrella crate for AMI, AGI, and combined usage.

## MSRV

The minimum supported Rust version is **1.83**.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-APACHE)
or [MIT License](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT) at your option.
