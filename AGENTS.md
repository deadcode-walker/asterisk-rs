# Repository Guidelines

## Project Overview

asterisk-rs is a Rust workspace providing async clients for the three Asterisk PBX integration protocols: AMI (Manager Interface), AGI (Gateway Interface), and ARI (REST Interface). All crates are async-only using tokio, target MSRV 1.83, and are dual-licensed MIT OR Apache-2.0.

## Architecture & Data Flow

```
asterisk-rs (umbrella, feature-gated re-exports)
  |
  +-- asterisk-rs-core (shared foundation)
  |     error.rs    -- Error, ConnectionError, AuthError, TimeoutError, ProtocolError
  |     event.rs    -- Event trait, EventBus<E>, EventSubscription<E> (tokio broadcast)
  |     config.rs   -- ReconnectPolicy (exponential backoff + jitter), ConnectionState enum
  |     auth.rs     -- Credentials (redacted Debug, never leaks secret)
  |
  +-- asterisk-rs-ami (TCP client, port 5038)
  |     codec.rs      -- tokio-util Decoder/Encoder for Key: Value\r\n\r\n framing
  |     action.rs     -- AmiAction trait + 116 typed action structs covering all Asterisk 23 actions
  |     response.rs   -- AmiResponse parsing, PendingActions (ActionID correlation via oneshot)
  |     event.rs      -- AmiEvent enum (161 typed variants + Unknown), implements core::Event
  |     connection.rs -- ConnectionManager: background task, reconnect loop, message dispatch
  |     client.rs     -- AmiClient builder, send_action<A>, MD5 challenge-response auth
  |
  +-- asterisk-rs-agi (TCP server, port 4573)
  |     server.rs     -- AgiServer<H: AgiHandler>: TCP listener, Semaphore concurrency
  |     handler.rs    -- AgiHandler trait (RPITIT, async fn in trait)
  |     request.rs    -- AgiRequest: parsed agi_* environment variables
  |     channel.rs    -- AgiChannel: all 47 typed AGI commands over split TCP stream
  |     command.rs    -- 47 command constants + format_command() with argument quoting
  |     response.rs   -- AgiResponse: parse "200 result=X (data) endpos=N"
  |
  +-- asterisk-rs-ari (HTTP + WebSocket, port 8088)
        client.rs     -- AriClient: reqwest REST + WsEventListener, Basic Auth
        config.rs     -- AriConfigBuilder: constructs base_url + ws_url
        websocket.rs  -- WsEventListener: background task, reconnect, JSON deserialization
        event.rs      -- AriEvent enum (all 43 typed variants, serde tagged on "type"), Channel/Bridge/Playback/Endpoint/Peer/ContactInfo types
        resources/    -- Handle pattern: ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle
                         + free functions (list, get, create, originate) per resource
```

**Dependency direction**: protocol crates depend on core; umbrella depends on all. Protocol crates never depend on each other.

**Concurrency model**: AMI and ARI spawn background tokio tasks for connection management and event dispatch. Communication between client API and background tasks uses `mpsc` (commands), `oneshot` (action responses), `watch` (connection state), and `broadcast` (events).

## Key Directories

| Path | Purpose |
|------|---------|
| `crates/asterisk-rs-core/src/` | Shared error types, event bus, reconnect policy, credentials |
| `crates/asterisk-rs-ami/src/` | AMI protocol: codec, actions, events, client, connection |
| `crates/asterisk-rs-agi/src/` | AGI protocol: server, handler trait, channel commands |
| `crates/asterisk-rs-ari/src/` | ARI protocol: REST client, WebSocket, events, resource handles |
| `crates/asterisk-rs-ari/src/resources/` | One module per ARI resource (channel, bridge, endpoint, etc.) |
| `crates/asterisk-rs/src/` | Umbrella crate with `#[cfg(feature)]` re-exports |
| `docs/src/` | mdBook user guide (ami/, agi/, ari/ subdirectories) |
| `.github/workflows/` | CI, security audit, docs deploy, release, coverage, semver checks |

## Development Commands

```sh
# build
cargo build --workspace

# test (all features)
cargo test --workspace --all-features

# test (no default features, validates feature gates)
cargo test --workspace --no-default-features

# lint
cargo clippy --workspace --all-targets --all-features -- -D warnings

# format (check)
cargo fmt --all -- --check

# format (apply)
cargo fmt --all

# generate rustdoc
cargo doc --workspace --all-features --no-deps

# build mdBook (requires mdbook installed)
mdbook build docs/
```

## Code Conventions & Common Patterns

### Workspace-Level Lints (enforced, not optional)

These are set in the root `Cargo.toml` and inherited by every crate via `[lints] workspace = true`:

- `unsafe_code = "forbid"` -- no unsafe anywhere
- `unwrap_used = "deny"` -- use `.expect("reason")` or proper error handling
- `enum_glob_use = "deny"` -- always qualify enum variants

### Formatting

- `rustfmt.toml`: edition 2021, `max_width = 100`
- `clippy.toml`: `msrv = "1.83"`, `cognitive-complexity-threshold = 25`

### Comment Style

- All lowercase, no trailing period
- Explain why, not what
- No filler: "robust", "comprehensive", "leverage", "ensure seamless" are banned

### Error Handling

Every crate defines its own error enum wrapping core errors via `thiserror`:

```
Core: Error { Connection, Auth, Timeout, Protocol, Io }
  AMI: AmiError { Connection, Auth, Timeout, Protocol, Io, ActionFailed, Disconnected, ResponseChannelClosed }
  AGI: AgiError { Io, ChannelHungUp, InvalidResponse, CommandFailed, Protocol }
  ARI: AriError { Http, WebSocket, Api, Json, Connection, Auth, Io, Disconnected, InvalidUrl }
```

Each crate re-exports `pub type Result<T> = std::result::Result<T, XxxError>;`.

### Builder Pattern

All clients and servers use builder pattern for configuration:

- `AmiClient::builder().host().port().credentials().build().await?` -- connect + login
- `AgiServer::builder().bind().handler().max_connections().build().await?` -- bind listener
- `AriConfigBuilder::new("app_name").host().port().username().password().build()?` -- then `AriClient::connect(config).await?`

Builders validate required fields in `.build()` and return `Result`.

### Event System

Protocol events implement `asterisk_rs_core::event::Event` (requires `Clone + Send + Sync + Debug + 'static`). Published via `EventBus<E>` (tokio broadcast). Consumed via `EventSubscription<E>::recv()` which handles lag by logging and skipping.

- AMI: `AmiEvent` enum with 161 typed variants + `Unknown { event_name, headers }`
- ARI: `AriEvent` enum with all 43 typed variants, serde `#[serde(tag = "type")]` + `#[serde(other)] Unknown`
- AGI: No event bus (synchronous request/response protocol)

### Reconnection

AMI and ARI use `ReconnectPolicy` from core:
- `ReconnectPolicy::exponential(initial, max)` -- default 1s initial, 60s max, jitter enabled
- `ReconnectPolicy::fixed(interval)` -- constant delay
- `ReconnectPolicy::none()` -- no retry

Background tasks manage reconnection state machine: `Disconnected -> Connecting -> Connected -> Reconnecting`.

### Handle Pattern (ARI)

ARI resources use handles that bundle resource ID + client reference:

```rust
let handle = ChannelHandle::new(channel.id, client.clone());
handle.answer().await?;
handle.play("sound:hello").await?;
handle.hangup(None).await?;
```

Handles are `Clone + Debug`. Operations construct REST paths from the embedded ID.

### Action Trait (AMI)

AMI actions implement `AmiAction`:

```rust
pub trait AmiAction {
    fn action_name(&self) -> &str;
    fn to_headers(&self) -> Vec<(String, String)>;
    fn to_message(&self) -> (String, RawAmiMessage); // default impl adds Action + ActionID
}
```

ActionIDs are globally unique via `AtomicU64` counter.

### Handler Trait (AGI)

AGI uses RPITIT (return position impl trait in trait, stable since Rust 1.75):

```rust
pub trait AgiHandler: Send + Sync + 'static {
    fn handle(&self, request: AgiRequest, channel: AgiChannel)
        -> impl Future<Output = Result<()>> + Send;
}
```

### Credentials Safety

`Credentials` has a custom `Debug` impl that redacts the secret field. Never derive `Debug` on types containing credentials.

## Important Files

| File | Role |
|------|------|
| `Cargo.toml` | Workspace root: MSRV, lints, shared deps |
| `crates/asterisk-rs-core/src/error.rs` | Core error hierarchy (all crates wrap these) |
| `crates/asterisk-rs-core/src/event.rs` | EventBus and Event trait (AMI + ARI pub/sub) |
| `crates/asterisk-rs-ami/src/codec.rs` | AMI wire protocol parser/serializer |
| `crates/asterisk-rs-ami/src/client.rs` | AmiClient public API + AmiClientBuilder |
| `crates/asterisk-rs-ami/src/connection.rs` | AMI background connection task |
| `crates/asterisk-rs-agi/src/handler.rs` | AgiHandler trait definition |
| `crates/asterisk-rs-agi/src/server.rs` | FastAGI server accept loop |
| `crates/asterisk-rs-ari/src/client.rs` | AriClient REST + WebSocket API |
| `crates/asterisk-rs-ari/src/event.rs` | AriEvent serde-tagged enum + supporting types |
| `crates/asterisk-rs-ari/src/resources/channel.rs` | ChannelHandle + OriginateParams |
| `crates/asterisk-rs/src/lib.rs` | Umbrella crate feature-gated re-exports |
| `deny.toml` | License and security policy for dependencies |

## Runtime & Tooling

- **Async runtime**: tokio (hard dependency, not runtime-agnostic)
- **TLS**: rustls (via reqwest and tokio-tungstenite features; no openssl dependency)
- **MSRV**: 1.83 (required for async fn in trait / RPITIT)
- **Edition**: 2021
- **License**: MIT OR Apache-2.0
- **Formatter**: rustfmt (run `cargo fmt --all`)
- **Linter**: clippy with `-D warnings` (run via clippy command above)
- **Security**: cargo-deny for license/advisory checks (`deny.toml`)
- **Releases**: release-plz (manual trigger via GitHub Actions `workflow_dispatch`)
- **Docs**: mdBook (user guide at `docs/`) + rustdoc (API reference)

## CI Matrix

| Job | Runs On | Toolchain | What |
|-----|---------|-----------|------|
| check | ubuntu | stable | `cargo check --workspace --all-targets --all-features` |
| fmt | ubuntu | nightly | `cargo fmt --all -- --check` |
| clippy | ubuntu | stable | `cargo clippy` with `-D warnings` |
| test | ubuntu/macos/windows | stable + 1.83 | `cargo test --workspace --all-features` |
| test-minimal | ubuntu | stable | `cargo test --workspace --no-default-features` |
| security | ubuntu | stable | Weekly + on Cargo.toml changes; cargo-deny + rustsec audit |
| coverage | ubuntu | stable | cargo-llvm-cov, uploads to codecov |
| semver | ubuntu | stable | cargo-semver-checks on PRs |
| docs | ubuntu | stable | rustdoc + mdbook, deploys to GitHub Pages |

## Testing

### Framework

Standard Rust `#[test]` with `#[cfg(test)]` modules embedded in source files. No external test frameworks (no proptest, mockito, etc.).

### Test Location & Count

Tests are co-located with source:

| File | Tests | Coverage |
|------|-------|----------|
| `asterisk-rs-ami/src/codec.rs` | 7 | Banner parsing, encode/decode, partial messages, size guard |
| `asterisk-rs-ami/src/response.rs` | 6 | Response parsing, PendingActions lifecycle |
| `asterisk-rs-ami/src/event.rs` | 3 | Event parsing, unknown events, non-event filtering |
| `asterisk-rs-agi/src/response.rs` | 7 | AGI response codes, data/endpos parsing |
| `asterisk-rs-agi/src/command.rs` | 5 | Command formatting, quoting, escaping |
| `asterisk-rs-ari/src/event.rs` | 4 | JSON deserialization, optional fields, unknown types |

### Test Patterns

- Construct test data inline (e.g., `RawAmiMessage { headers: vec![...] }`, `BytesMut::from(...)`, JSON string literals)
- Use `.expect("reason")` for test assertions on Results (allowed in test cfg)
- Pattern match on enum variants with `panic!("expected X")` in else branches
- No shared test fixtures or helper functions

### Coverage Gaps

- `asterisk-rs-core`: zero tests (error, event, config, auth modules)
- Client/connection logic: no tests for AmiClient, AriClient, ConnectionManager
- Network layer: no mock servers, no integration tests
- AGI server accept loop and handler dispatch: untested

### Running Tests

```sh
# all tests
cargo test --workspace --all-features

# specific crate
cargo test -p asterisk-rs-ami

# specific test
cargo test -p asterisk-rs-ami codec::tests::decode_banner_and_response
```

### Examples

Located in each crate's `examples/` directory (not workspace root):

| Example | Crate | Demonstrates |
|---------|-------|-------------|
| `ami_originate.rs` | asterisk-rs-ami | Builder, OriginateAction, response handling |
| `ami_events.rs` | asterisk-rs-ami | Event subscription loop |
| `agi_server.rs` | asterisk-rs-agi | AgiHandler impl, channel operations |
| `ari_stasis_app.rs` | asterisk-rs-ari | Stasis event loop, ChannelHandle |
| `ari_bridge.rs` | asterisk-rs-ari | Bridge creation, channel origination |

All examples require a running Asterisk instance and use `tracing_subscriber` (dev-dependency).

## Dependency Policy

Allowed licenses: MIT, Apache-2.0, BSD-2-Clause, BSD-3-Clause, ISC, Unicode-3.0, Unicode-DFS-2016, Zlib. Copyleft (GPL, LGPL, AGPL) is denied. Enforced via `deny.toml` and cargo-deny in CI.
