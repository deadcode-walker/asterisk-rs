# AMI API reference

The public Rust API is documented in
[`asterisk_rs_ami`](https://docs.rs/asterisk-rs-ami/0.8/asterisk_rs_ami/). Rustdoc is the canonical
inventory of actions, events, builders, errors, and methods; this guide explains how those APIs fit
together without duplicating a generated symbol table.

## Find the right API

- [`AmiClient`](https://docs.rs/asterisk-rs-ami/0.8/asterisk_rs_ami/struct.AmiClient.html) owns
  connection, authentication, subscriptions, action correlation, and shutdown.
- [`action`](https://docs.rs/asterisk-rs-ami/0.8/asterisk_rs_ami/action/index.html) contains typed AMI
  actions. Use `send` for one response and `send_collecting` for Asterisk actions that terminate with
  a completion event.
- [`AmiEvent`](https://docs.rs/asterisk-rs-ami/0.8/asterisk_rs_ami/event/enum.AmiEvent.html) contains
  modeled events and retains unknown events for forward-compatible handling.
- [`AmiResponse`](https://docs.rs/asterisk-rs-ami/0.8/asterisk_rs_ami/response/struct.AmiResponse.html)
  exposes parsed response fields and output from `Response: Follows`.

Start with [connection and authentication](connection.md), then use [events](events.md) for
subscription and lag behavior. The crate examples show complete Tokio programs.
