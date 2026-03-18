# FastAGI Server

## Binding

The server binds a TCP listener and dispatches each connection to your handler.
Asterisk connects via the `AGI()` dialplan application:

```ini
exten => 100,1,AGI(agi://your-server:4573)
```

## Handler Trait

```rust,ignore
pub trait AgiHandler: Send + Sync + 'static {
    fn handle(&self, request: AgiRequest, channel: AgiChannel)
        -> impl Future<Output = Result<()>> + Send;
}
```

The handler receives the AGI request (parsed environment variables from Asterisk)
and a channel for sending commands back.

## Request Environment

`AgiRequest` contains the `agi_*` variables sent by Asterisk at connection start:
channel name, caller ID, called extension, context, language, etc.

## Channel Commands

`AgiChannel` provides typed methods for all 47 AGI commands: `answer`, `hangup`,
`stream_file`, `get_data`, `say_digits`, `record_file`, `database_get`,
`speech_create`, and more. See [Reference](./reference.md).

## Concurrency

Limit concurrent connections with `max_connections`:

```rust,ignore
let (server, _shutdown) = AgiServer::builder()
    .bind("0.0.0.0:4573")
    .handler(MyHandler)
    .max_connections(50)
    .build()
    .await?;
```

## Graceful Shutdown

`build()` returns a `ShutdownHandle` that stops the accept loop:

```rust,ignore
let (server, shutdown) = AgiServer::builder()
    .bind("0.0.0.0:4573")
    .handler(MyHandler)
    .build()
    .await?;

// stop accepting after ctrl-c
tokio::spawn(async move {
    tokio::signal::ctrl_c().await.ok();
    shutdown.shutdown();
});

server.run().await?;
```
