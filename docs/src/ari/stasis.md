# Stasis Applications

ARI routes calls to your application via the `Stasis()` dialplan application:

```ini
exten => 100,1,Stasis(my-app,arg1,arg2)
```

## Event Stream

Events arrive via WebSocket as `AriMessage` structs containing metadata
and a typed `AriEvent`:

```rust,ignore
use asterisk_rs_ari::event::{AriEvent, AriMessage};

let mut events = client.subscribe();
while let Some(msg) = events.recv().await {
    println!("app={} time={}", msg.application, msg.timestamp);
    match msg.event {
        AriEvent::StasisStart { channel, args, .. } => {
            println!("call from {} with args {:?}", channel.name, args);
        }
        AriEvent::StasisEnd { channel } => {
            println!("call ended: {}", channel.name);
        }
        _ => {}
    }
}
```

## Filtered Subscriptions

```rust,ignore
let mut calls = client.subscribe_filtered(|msg| {
    matches!(msg.event, AriEvent::StasisStart { .. } | AriEvent::StasisEnd { .. })
});
```

## Event Metadata

Every `AriMessage` carries:
- `application` — the Stasis app that received the event
- `timestamp` — ISO 8601 when the event was created
- `asterisk_id` — unique Asterisk instance ID (for clusters)
- `event` — the typed `AriEvent` payload

See [Reference](./reference.md) for the complete event list.
