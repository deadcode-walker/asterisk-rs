# Resources

ARI organizes control operations around resource types. Each resource type has
a handle struct that wraps a reference to the `AriClient` and provides typed
methods for interacting with that resource.

## Handle Pattern

Handles are lightweight wrappers that hold a client reference and a resource ID.
They do not own the resource -- they provide a typed interface to it:

```rust,no_run
use asterisk_ari::AriClient;
use asterisk_ari::resources::channel::ChannelHandle;

async fn example(client: &AriClient) {
    let ch = ChannelHandle::new(client, "1234567.42");
    ch.answer().await.unwrap();
    ch.play("sound:hello-world", "en", 0, 0).await.unwrap();
    ch.hangup().await.unwrap();
}
```

## Channels

`ChannelHandle` controls a single channel identified by its ID.

| Method | Description |
|--------|-------------|
| `answer()` | Answer the channel |
| `hangup()` | Hang up the channel |
| `play(media, lang, offsetms, skipms)` | Play media to the channel |
| `record(name, format, max_seconds, max_silence, beep)` | Record from the channel |
| `mute(direction)` | Mute the channel |
| `unmute(direction)` | Unmute the channel |

Channels can be originated using the module-level originate function with
`OriginateParams`:

```rust,no_run
use asterisk_ari::AriClient;
use asterisk_ari::resources::channel::OriginateParams;

async fn originate(client: &AriClient) {
    let params = OriginateParams {
        endpoint: "PJSIP/6001".to_owned(),
        extension: Some("100".to_owned()),
        context: Some("default".to_owned()),
        priority: Some(1),
        app: None,
        app_args: None,
        caller_id: Some("5551234".to_owned()),
        timeout: Some(30),
        variables: None,
    };
    // channel::originate(client, &params).await.unwrap();
}
```

## Bridges

`BridgeHandle` controls a mixing bridge for connecting channels together.

| Method | Description |
|--------|-------------|
| `add_channel(channel_id)` | Add a channel to the bridge |
| `remove_channel(channel_id)` | Remove a channel from the bridge |
| `play(media, lang, offsetms, skipms)` | Play media to the bridge |
| `record(name, format, max_seconds, max_silence, beep)` | Record the bridge |
| `destroy()` | Destroy the bridge |

Module-level functions: `bridge::create()`, `bridge::list()`, `bridge::get()`.

### Example: Bridging Two Channels

```rust,no_run
use asterisk_ari::{AriClient, AriEvent};
use asterisk_ari::resources::bridge;
use asterisk_ari::resources::bridge::BridgeHandle;
use asterisk_ari::resources::channel::ChannelHandle;

async fn bridge_calls(
    client: &AriClient,
    channel_a: &str,
    channel_b: &str,
) -> Result<(), Box<dyn std::error::Error>> {
    // create a mixing bridge
    let bridge_data = bridge::create(client, "mixing", "my-bridge").await?;
    let br = BridgeHandle::new(client, &bridge_data.id);

    // answer both channels
    let ch_a = ChannelHandle::new(client, channel_a);
    let ch_b = ChannelHandle::new(client, channel_b);
    ch_a.answer().await?;
    ch_b.answer().await?;

    // add both to the bridge
    br.add_channel(channel_a).await?;
    br.add_channel(channel_b).await?;

    Ok(())
}
```

## Playbacks

`PlaybackHandle` controls an active media playback. Playbacks are created by
calling `play()` on a channel or bridge handle. The handle can be used to
control or query the playback.

## Recordings

`RecordingHandle` controls a live recording started on a channel or bridge.

## Other Resources

The crate also provides access to:

| Module | Description |
|--------|-------------|
| `application` | List and inspect Stasis applications |
| `device_state` | Query and set device state |
| `endpoint` | List and inspect endpoints |
| `mailbox` | Manage mailboxes |
| `sound` | List available sound files |

These are accessed via module-level functions that take an `&AriClient` and
return typed response structs.
