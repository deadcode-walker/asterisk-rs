# FastAGI Server

## Binding

The server binds a TCP listener and dispatches each connection to your handler.
Asterisk connects via the `AGI()` dialplan application:

```ini
exten => 100,1,AGI(agi://your-server:4573)
```

The builder defaults to `127.0.0.1:4573`. FastAGI has no native peer authentication, so an external
bind is rejected unless `allow_external_bind(true)` is explicit. External listeners must be
isolated with a private network, firewall allowlist, or authenticated TLS proxy.

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

`AgiChannel` provides typed async methods for every AGI command: `answer`, `hangup`,
`stream_file`, `get_data`, `say_digits`, `record_file`, `database_get`,
`speech_create`, and more. See [API reference](./reference.md) for canonical rustdoc links.

Command round trips have no deadline by default because `WAIT FOR DIGIT -1`, dial applications,
recording, and speech operations can wait indefinitely. Applications that need a bound can call
`channel.set_command_timeout(Some(duration))`; expiry poisons the channel because a late response
cannot be correlated safely. Pass `None` to disable the deadline again.

## Concurrency

Limit concurrent connections with `max_connections`:

```rust,ignore
let (server, _shutdown) = AgiServer::builder()
    .bind("0.0.0.0:4573")
    .allow_external_bind(true)
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
    .allow_external_bind(true)
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
