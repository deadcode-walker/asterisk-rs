# asterisk-rs architecture

asterisk-rs is a Tokio-only Rust workspace for the three Asterisk application integration
protocols. It publishes one shared foundation crate, one crate per protocol, and an umbrella crate
that feature-gates re-exports and higher-level composition.

## Workspace map

| Path | Owns | Does not own |
|---|---|---|
| `crates/asterisk-rs-core` | shared errors, credentials, domain types, reconnect policy, event bus | protocol framing or transport |
| `crates/asterisk-rs-ami` | AMI TCP framing, actions, typed events, connection lifecycle, call tracking | AGI or ARI behavior |
| `crates/asterisk-rs-agi` | FastAGI server, request parsing, command formatting, channel session | background event dispatch |
| `crates/asterisk-rs-ari` | ARI HTTP/WebSocket transports, events, resources, media, outbound WS server | AMI or AGI behavior |
| `crates/asterisk-rs` | feature-gated re-exports and `Pbx` composition | protocol implementation |
| `tests` | unit, mock-boundary, and live Asterisk evidence | production implementation |

## Dependency direction

```text
asterisk-rs  --->  ami  --->  core
     |             agi  --->  core
     +---------->  ari  --->  core

tests  --->  all publishable crates
```

Protocol crates must not depend on one another. Shared protocol-neutral facts move downward to
core; cross-protocol orchestration moves upward to the umbrella crate or an application.

## Useful paths

| Outcome | Trace from | Through | Observable boundary |
|---|---|---|---|
| AMI connect, action, and event correlation | `asterisk-rs-ami/src/client.rs` | `connection.rs`, `codec.rs`, `response.rs` | AMI mock server or live port 5038 |
| FastAGI session and command exchange | `asterisk-rs-agi/src/server.rs` | `request.rs`, `handler.rs`, `channel.rs`, `response.rs` | mock AGI peer or live FastAGI session |
| ARI REST and event lifecycle | `asterisk-rs-ari/src/client.rs` | `transport.rs`, `websocket.rs`, `resources/` | ARI HTTP/WS mock or live port 8088 |
| Unified/outbound ARI request correlation | `asterisk-rs-ari/src/ws_transport.rs` | `ws_proto.rs`, `server.rs` | typed WebSocket request/response IDs |
| Media WebSocket control and audio | `asterisk-rs-ari/src/media.rs` | explicit TLS connector and bounded socket actor | chan_websocket fixture or live Asterisk |
| Cross-protocol PBX convenience | `asterisk-rs/src/pbx.rs` | protocol crate public APIs only | external behavior tests in `tests/` |

Start with the named entry point and follow one complete path to its mock or live boundary before
changing a shared type, retry policy, lifecycle rule, or public API.

## Runtime flows

AMI uses a background connection manager. Client commands travel over `mpsc`, action results return
over `oneshot`, connection state is observed through `watch`, and events fan out through `broadcast`.
The codec owns `Key: Value\r\n\r\n` framing and ActionID owns request correlation.

AGI accepts TCP sessions behind a semaphore. Each session parses the `agi_*` environment into an
`AgiRequest`, hands split I/O to an `AgiChannel`, and serializes one command/response exchange at a
time. There is no event bus or reconnect manager because Asterisk owns the session.

ARI selects HTTP plus event WebSocket or a unified WebSocket transport. REST/WebSocket correlation
belongs to the transport; typed resources belong under `resources/`. Pending resource factories
pre-generate IDs and subscribe before creation so the first lifecycle event cannot race the caller.

## Cross-cutting rules

- Untrusted wire data is parsed at codecs, request parsers, or serde boundaries before use.
- Errors remain protocol-specific at public APIs and wrap shared core errors where appropriate.
- Secrets use redacted Debug implementations and zeroization where stored.
- Reconnect loops use bounded backoff, observable connection state, and explicit shutdown.
- Resource handles contain an ID plus a cloneable client; free functions remain available for
  stateless resource operations.
- Generated mdBook reference tables derive from Rust source via `docs/generate.py`.

## Proof boundaries

- Unit tests prove parsing, serialization, actions, events, types, and media protocol facts.
- Mock integration tests prove concurrency and transport lifecycle without external services.
- Live integration tests prove behavior against Asterisk and run serially where PBX state is shared.

## Deliberate absences

- No runtime abstraction: all public async behavior is designed for Tokio.
- No unsafe code: the workspace forbids it.
- No protocol-to-protocol dependencies: composition belongs above protocol crates.
- No Bazel graph: Cargo is required for publishing and current native feedback is fast. Reconsider
  only for a measured multi-language, native-build, remote-execution, or cross-platform drift problem.
- No internal tests in production modules: external tests exercise public and protocol boundaries.
