# asterisk-rs architecture

Status: active architecture and ownership map. Protocol maintainers own it and revalidate it when
runtime flows, public boundaries, commands, supported targets, or delivery policy change.

## System and useful path

asterisk-rs is a Tokio-only Rust workspace that lets applications integrate with Asterisk through
AMI, FastAGI, and ARI without sharing protocol implementation between those surfaces. It publishes
one shared foundation crate, one crate per protocol, and an umbrella crate for feature-gated
re-exports and deliberate cross-protocol composition.

A representative AMI path starts at `AmiClient`, validates and admits an action, passes it to the
connection actor, encodes it through the AMI codec, correlates the response by ActionID, and returns a
typed result. Observable proof ends at the mock AMI peer or a live Asterisk port 5038. Timeout and
cancellation semantics distinguish definitely-unsent work from an indeterminate post-write outcome.

| Outcome | Entry and path | Observable boundary |
|---|---|---|
| AMI action/event lifecycle | `asterisk-rs-ami/src/client.rs` → `connection.rs` → `codec.rs`/`response.rs` | AMI mock or live port 5038 |
| FastAGI session/command | `asterisk-rs-agi/src/server.rs` → `request.rs`/`channel.rs`/`response.rs` | mock peer or live FastAGI session |
| ARI REST/event lifecycle | `asterisk-rs-ari/src/client.rs` → `transport.rs`/`websocket.rs` → `resources/` | HTTP/WS mock or live port 8088 |
| Unified/outbound ARI correlation | `asterisk-rs-ari/src/ws_transport.rs` → `ws_proto.rs`/`server.rs` | typed WebSocket request/response IDs |
| Media WebSocket control/audio | `asterisk-rs-ari/src/media.rs` → bounded socket actor and explicit TLS connector | chan_websocket fixture or live Asterisk |
| Cross-protocol convenience | `asterisk-rs/src/pbx.rs` → protocol crate public APIs | external behavior tests in `tests/` |

## Code map and ownership

| Path | Responsibility and public boundary | Mutable-state owner |
|---|---|---|
| `crates/asterisk-rs-core` | protocol-neutral errors, credentials, domain types, reconnect policy, event bus | each constructed value or event bus |
| `crates/asterisk-rs-ami` | AMI TCP framing, actions, typed events, lifecycle, tracking | connection actor and tracker tasks |
| `crates/asterisk-rs-agi` | FastAGI listener, prelude parsing, commands, channel session | server task and one channel session |
| `crates/asterisk-rs-ari` | HTTP/WS transports, events, resources, media, outbound server | transport/media/server actors |
| `crates/asterisk-rs` | feature-gated re-exports and `Pbx` composition | composed protocol handles |
| `tests` | unit, mock-boundary, and live-Asterisk evidence | isolated fixtures and selected PBX |
| `docs/generate.py` | generated mdBook protocol/type references | generated files under `docs/src/` |

## Dependency and cross-cutting boundaries

```text
asterisk-rs  --->  ami  --->  core
     |             agi  --->  core
     +---------->  ari  --->  core

tests  --->  every publishable crate
```

Protocol crates never depend on peers. Shared protocol-neutral facts move down to core;
cross-protocol behavior moves up to the umbrella crate or an application. `scripts/check_harness.py`
enforces peer-dependency, unsafe-lint, instruction, plan, link, and external-test boundaries.

Untrusted wire data is parsed at codec/request/serde boundaries before effects. Protocol APIs own
their errors. Credentials remain redacted and zeroized where stored. Existing bounded lifecycle
controls are owned by their protocol actors; unresolved cross-protocol gaps remain explicit in the
active modernization plan. Generated references derive from Rust source; do not create a competing
handwritten table.

## Toolchain and evidence authority

| Concern | Single authority | Focused/complete evidence | Delivery boundary |
|---|---|---|---|
| compatibility and exact compiler | workspace `rust-version`, `rust-toolchain.toml`, `clippy.toml` | `just check`, `just msrv` | Linux/macOS/Windows CI |
| dependency/build graph | `Cargo.toml` and `Cargo.lock` | Cargo feature matrix, cargo-deny, cargo-shear | crates.io packages |
| public commands | `justfile` | `just --list`, recipe execution | contributors, agents, CI |
| protocol behavior | Rust source and external `tests` package | focused tests, `just ci`, `just live` | mock boundary or isolated Asterisk 22 |
| public documentation | rustdoc, `docs/src/`, `docs/generate.py` | `just docs`, `just docs-check` | GitHub Pages and published crates |
| release identity/changelogs | `release-plz.toml`, conventional commits | blocking CI and release PR review | protected GitHub/crates.io environments |
| workflow policy | `.github/workflows/` | `just workflows` and aggregate CI | GitHub repository settings |

The decision brief owns why these tools were chosen. Each added layer needs a measured gap, owner,
rollback/removal path, and recheck trigger; configuration does not duplicate implementation policy.

## Deliberate absences and freshness

- No runtime abstraction or synchronous client: Tokio is the supported async environment.
- No unsafe code and no production `#[cfg(test)]` modules.
- No Bazel, Nix, second resolver, second task runner, second CI provider, private agent manual,
  mandatory browser stack, telemetry stack, model grader, or multi-agent controller. Reconsider only
  after a measured requirement that current Cargo/Just/tests/logging cannot satisfy.
- No generic release profile for published libraries; downstream applications own final codegen.

Protocol maintainers recheck this map when crate ownership, dependency direction, runtime actors,
supported Asterisk/Rust/OS targets, command recipes, generated ownership, or release delivery changes.
