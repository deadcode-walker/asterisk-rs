# Events

AMI delivers real-time events as things happen in Asterisk. All 161 event types
are parsed into typed `AmiEvent` variants. See [Reference](./reference.md) for
the complete list.

## Subscribing

```rust,ignore
let mut sub = client.subscribe();

while let Some(event) = sub.recv().await {
    println!("{}: {}", event.event_name(), event.channel().unwrap_or("n/a"));
}
```

## Filtered Subscriptions

Subscribe to specific event types without processing every event:

```rust,ignore
let mut hangups = client.subscribe_filtered(|e| {
    e.event_name() == "Hangup"
});

while let Some(event) = hangups.recv().await {
    if let AmiEvent::Hangup { channel, cause, cause_txt, .. } = event {
        println!("hangup on {channel}: {cause} ({cause_txt})");
    }
}
```

## Event-Generating Actions

Actions like `Status`, `CoreShowChannels`, and `QueueStatus` return results
as a sequence of events. Use `send_collecting` to gather them:

```rust,ignore
use asterisk_rs_ami::action::StatusAction;

let result = client.send_collecting(&StatusAction { channel: None }).await?;
println!("got {} channel status events", result.events.len());
for event in &result.events {
    println!("  {}", event.event_name());
}
```

## Common Accessors

Every `AmiEvent` has:
- `event_name()` — the raw event name string
- `channel()` — the associated channel name, if any
- `unique_id()` — the unique channel identifier, if any

## Unknown Events

Events not covered by typed variants arrive as `AmiEvent::Unknown`:

```rust,ignore
if let AmiEvent::Unknown { event_name, headers } = event {
    println!("unhandled: {event_name}");
    for (k, v) in &headers {
        println!("  {k}: {v}");
    }
}
```
