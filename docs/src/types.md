# Domain types

Shared Asterisk constants are modeled in
[`asterisk_rs_core::types`](https://docs.rs/asterisk-rs-core/0.8/asterisk_rs_core/types/index.html).
Rustdoc is the canonical inventory of types, variants, numeric conversions, and string parsing.

Use these types when a protocol boundary has a closed or meaningfully classified value set:

- `HangupCause` for Q.850/Q.931 cause codes;
- `ChannelState`, `DeviceState`, and `ExtensionState` for observed state;
- `DialStatus` and `CdrDisposition` for call outcomes;
- `PeerStatus` and `QueueStrategy` for peer and queue state;
- `AgiStatus` for numeric AGI response status.

Unknown or combined wire values are preserved where the protocol requires forward compatibility.
Consult each type's rustdoc for its exact conversion behavior rather than matching a copied table.
