# asterisk-rs-agi

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-agi)](https://crates.io/crates/asterisk-rs-agi)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-agi)](https://docs.rs/asterisk-rs-agi)

Async Rust FastAGI server for the Asterisk Gateway Interface. Answer calls,
collect DTMF, play prompts, query databases, and control call flow.

## Example

```rust,ignore
use asterisk_rs_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel};

struct MyIvr;

impl AgiHandler for MyIvr {
    async fn handle(&self, req: AgiRequest, mut ch: AgiChannel)
        -> asterisk_rs_agi::error::Result<()>
    {
        ch.answer().await?;
        ch.stream_file("welcome", "").await?;
        let input = ch.get_data("enter-account", 5000, 6).await?;
        ch.verbose(&format!("caller entered: {:?}", input), 1).await?;
        ch.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, _shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyIvr)
        .max_connections(100)
        .build()
        .await?;

    server.run().await?;
    Ok(())
}
```

## Capabilities

- Every AGI command with typed async methods
- Handler trait using native async fn (RPITIT, no macro needed)
- Request environment parsing from Asterisk
- Configurable concurrency limits via semaphore
- Graceful shutdown via `ShutdownHandle`
- Argument quoting and escaping for special characters

## Documentation

- [API Reference](https://docs.rs/asterisk-rs-agi)
- [User Guide](https://deadcode-walker.github.io/asterisk-rs/)

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.83. MIT/Apache-2.0.
