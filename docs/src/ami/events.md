# Events

AMI delivers real-time events as things happen in the Asterisk system. The
crate parses these into typed `AmiEvent` variants.

## Subscribing

Call `client.subscribe()` to get an `EventSubscription` that receives events
from the internal broadcast channel:

```rust,no_run
use asterisk_ami::{AmiClient, AmiEvent};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client = AmiClient::builder()
        .host("127.0.0.1")
        .credentials("admin", "secret")
        .build()
        .await?;

    let mut sub = client.subscribe();

    while let Ok(event) = sub.recv().await {
        match event {
            AmiEvent::NewChannel(data) => {
                println!("new channel: {:?}", data);
            }
            AmiEvent::Hangup(data) => {
                println!("hangup: {:?}", data);
            }
            _ => {}
        }
    }

    Ok(())
}
```

Multiple subscribers can exist simultaneously. Each receives a copy of every
event. If a subscriber falls behind, events are dropped from its buffer.

## Event Variants

| Variant | Fired when |
|---------|------------|
| `NewChannel` | A new channel is created |
| `Hangup` | A channel is hung up |
| `Newstate` | A channel changes state (ringing, up, etc.) |
| `DialBegin` | An outbound dial attempt starts |
| `DialEnd` | An outbound dial attempt completes |
| `DtmfBegin` | A DTMF digit press starts |
| `DtmfEnd` | A DTMF digit press ends |
| `FullyBooted` | Asterisk has finished starting up |
| `PeerStatus` | A SIP/PJSIP peer changes registration status |
| `BridgeCreate` | A bridge is created |
| `BridgeDestroy` | A bridge is destroyed |
| `BridgeEnter` | A channel enters a bridge |
| `BridgeLeave` | A channel leaves a bridge |
| `Unknown` | Any event not covered above |

## Accessing Event Data

Each variant carries a struct with the relevant fields parsed from the AMI
message headers. Common accessors available on `AmiEvent`:

- `event_name()` -- the raw event name string
- `channel()` -- the associated channel name, if any
- `unique_id()` -- the unique channel identifier, if any

## Unknown Events

Asterisk modules can generate custom events not covered by the typed variants.
These arrive as `AmiEvent::Unknown`, which preserves all raw headers so you can
inspect them manually:

```rust,no_run
# use asterisk_ami::AmiEvent;
# fn handle(event: AmiEvent) {
if let AmiEvent::Unknown(raw) = event {
    let name = raw.get("Event").unwrap_or_default();
    println!("unhandled event: {}", name);
}
# }
```

## Filtering

For high-volume systems, filter events early to avoid unnecessary processing:

```rust,no_run
# use asterisk_ami::AmiEvent;
# async fn example(mut sub: asterisk_rs_core::event::EventSubscription<AmiEvent>) {
while let Ok(event) = sub.recv().await {
    // only process call lifecycle events
    match &event {
        AmiEvent::NewChannel(_)
        | AmiEvent::Hangup(_)
        | AmiEvent::DialBegin(_)
        | AmiEvent::DialEnd(_) => {
            process_call_event(event);
        }
        _ => {} // discard
    }
}
# }
# fn process_call_event(_: AmiEvent) {}
```
