# asterisk-rs-agi

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-agi)](https://crates.io/crates/asterisk-rs-agi)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-agi)](https://docs.rs/asterisk-rs-agi)
[![license](https://img.shields.io/crates/l/asterisk-rs-agi)](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT)

Async Rust FastAGI server for Asterisk Gateway Interface.

AGI allows external programs to control Asterisk dialplan execution over a TCP
connection (FastAGI, port 4573). This crate provides a typed, async server
built on Tokio that dispatches incoming AGI sessions to a user-defined handler.

Features:

- `AgiHandler` trait using async fn in trait (RPITIT)
- 47 typed AGI commands (`answer`, `stream_file`, `get_data`, `hangup`, etc.)
- `AgiChannel` for sending commands and reading responses within a session
- `AgiRequest` with parsed `agi_*` environment variables
- Configurable concurrency via `max_connections`

## Install

```sh
cargo add asterisk-rs-agi
```

## Usage

```rust,no_run
use asterisk_rs_agi::{AgiChannel, AgiHandler, AgiRequest, AgiServer};

struct MyHandler;

impl AgiHandler for MyHandler {
    async fn handle(
        &self,
        request: AgiRequest,
        mut channel: AgiChannel,
    ) -> asterisk_rs_agi::error::Result<()> {
        channel.answer().await?;
        channel.stream_file("hello-world", "#").await?;
        channel.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let server = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyHandler)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

See [`examples/agi_server.rs`](examples/agi_server.rs) for a more complete example.

## Part of asterisk-rs

This crate is part of the [`asterisk-rs`](https://github.com/deadcode-walker/asterisk-rs) workspace.

## MSRV

The minimum supported Rust version is **1.83**.

## License

Licensed under either of [Apache License, Version 2.0](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-APACHE) or
[MIT License](https://github.com/deadcode-walker/asterisk-rs/blob/main/LICENSE-MIT) at your option.
