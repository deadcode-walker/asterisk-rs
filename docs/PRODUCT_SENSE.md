# Product sense

asterisk-rs serves Rust developers who need typed, asynchronous access to Asterisk without writing
wire framing, reconnect state machines, or ad hoc JSON/resource code.

## Product principles

- Make correct protocol behavior the easy default.
- Preserve typed escape hatches for Asterisk versions and events the library does not yet know.
- Keep low-level protocol crates independently usable; the umbrella crate adds convenience rather
  than hiding protocol facts.
- Prefer explicit timeouts, shutdown, transport modes, and failure states over implicit background
  behavior.
- Maintain MSRV and semver promises as tested compatibility boundaries.

## Non-goals

- Operating or configuring Asterisk itself.
- Providing a SIP stack, media server, dialplan engine, or runtime-independent async abstraction.
- Hiding all AMI, AGI, or ARI concepts behind one lowest-common-denominator API.

Exact user-visible behavior is documented under `docs/src/` and indexed in `product-specs/`.
