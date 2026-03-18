# AGI (Asterisk Gateway Interface)

AGI allows external programs to control Asterisk dialplan execution.
This crate implements a FastAGI TCP server that accepts connections
from Asterisk and dispatches them to a handler.

## Quick Start

```rust,ignore
use asterisk_rs_agi::{AgiServer, AgiHandler, AgiRequest, AgiChannel};

struct MyHandler;

impl AgiHandler for MyHandler {
    async fn handle(&self, request: AgiRequest, mut channel: AgiChannel)
        -> asterisk_rs_agi::error::Result<()>
    {
        channel.answer().await?;
        channel.stream_file("hello-world", "").await?;
        channel.hangup(None).await?;
        Ok(())
    }
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let (server, _shutdown) = AgiServer::builder()
        .bind("0.0.0.0:4573")
        .handler(MyHandler)
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
- Configurable concurrency limits
- Graceful shutdown via `ShutdownHandle`

See [FastAGI Server](./fastagi.md) for server details and
[Reference](./reference.md) for the complete command list.
