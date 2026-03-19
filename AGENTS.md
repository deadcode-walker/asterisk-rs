# Repository Guidelines

## Project Overview

asterisk-rs is a Rust workspace providing async clients for the three Asterisk PBX integration protocols: AMI (Manager Interface), AGI (Gateway Interface), and ARI (REST Interface). All crates are async-only using tokio, target MSRV 1.83, and are dual-licensed MIT OR Apache-2.0.

## Architecture & Data Flow

```
asterisk-rs (umbrella, feature-gated re-exports)
  |  pbx.rs        -- Pbx high-level call abstraction wrapping AmiClient + CallTracker
  |
  +-- asterisk-rs-core (shared foundation)
  |     error.rs    -- Error, ConnectionError, AuthError, TimeoutError, ProtocolError
  |     event.rs    -- Event trait, EventBus<E>, EventSubscription<E>, FilteredSubscription<E>
  |     config.rs   -- ReconnectPolicy (exponential backoff + jitter), ConnectionState enum
  |     auth.rs     -- Credentials (redacted Debug, never leaks secret)
  |     types.rs    -- domain constants (HangupCause, ChannelState, DeviceState, etc.)
  |
  +-- asterisk-rs-ami (TCP client, port 5038)
  |     codec.rs      -- tokio-util Decoder/Encoder for Key: Value\r\n\r\n framing + ChanVariable extraction
  |     action.rs     -- AmiAction trait + typed action structs for all Asterisk 23 actions
  |     response.rs   -- AmiResponse, EventListResponse, PendingActions (ActionID correlation)
  |     event.rs      -- AmiEvent enum (typed variants + Unknown), implements core::Event
  |     connection.rs -- ConnectionManager: background task, reconnect loop, message dispatch, keep-alive ping
  |     client.rs     -- AmiClient builder, send_action<A>, MD5 challenge-response auth, ping_interval config
  |     tracker.rs    -- CallTracker: correlates AMI events by UniqueID into CompletedCall records
  |
  +-- asterisk-rs-agi (TCP server, port 4573)
  |     server.rs     -- AgiServer<H: AgiHandler>: TCP listener, Semaphore concurrency
  |     handler.rs    -- AgiHandler trait (RPITIT, async fn in trait)
  |     request.rs    -- AgiRequest: parsed agi_* environment variables
  |     channel.rs    -- AgiChannel: typed AGI commands over split TCP stream
  |     command.rs    -- command constants + format_command() with argument quoting
  |     response.rs   -- AgiResponse: parse "200 result=X (data) endpos=N"
  |
  +-- asterisk-rs-ari (HTTP + WebSocket, port 8088)
        client.rs     -- AriClient: transport-abstracted REST + event subscription
        config.rs     -- AriConfigBuilder: base_url, ws_url, TransportMode (Http|WebSocket)
        transport.rs  -- TransportInner enum (HttpTransport | WsTransport), TransportResponse
        ws_transport.rs -- WsTransport: unified REST + events over single WebSocket, request correlation
        websocket.rs  -- WsEventListener: background task for event-only WS (HTTP transport mode)
        event.rs      -- AriEvent enum (serde tagged on "type"), AriMessage wrapper with metadata
        pending.rs    -- PendingChannel/Bridge/Playback: race-free resource creation with pre-event subscription
        server.rs     -- AriServer: outbound WebSocket server accepting connections from Asterisk 22+
        media.rs      -- MediaChannel: chan_websocket audio exchange, MediaEvent/MediaCommand typed protocol
        resources/    -- Handle pattern: ChannelHandle, BridgeHandle, PlaybackHandle, RecordingHandle
                         + free functions (list, get, create, originate, external_media) per resource

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
| `tests/` | External test crate: unit, mock integration, live integration tests |
| `tests/src/mock/` | Mock servers: MockAmiServer, MockAriServer, MockAgiClient |
| `docs/src/` | mdBook user guide (ami/, agi/, ari/ subdirectories) |
| `.github/workflows/` | CI, security audit, docs deploy, release, coverage, semver checks |

## Development Commands

```sh
# build
cargo build --workspace

# test (unit + mock, no network needed)
cargo test -p asterisk-rs-tests --test unit --test mock_integration

# test (live, requires running Asterisk — see tests/docker-compose.yml)
cargo test-live

# test (workspace crates, all features)
cargo test --workspace --all-features --exclude asterisk-rs-tests

# test (no default features, validates feature gates)
cargo test --workspace --no-default-features --exclude asterisk-rs-tests

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

- `AmiClient::builder().host().port().credentials().ping_interval().build().await?` -- connect + login
- `AgiServer::builder().bind().handler().max_connections().build().await?` -- bind listener, returns `(AgiServer, ShutdownHandle)`
- `AriConfigBuilder::new("app_name").host().port().username().password().transport(TransportMode::WebSocket).build()?` -- then `AriClient::connect(config).await?`
- `AriServer::builder().bind(addr).build().await?` -- returns `(AriServer, ShutdownHandle)` for outbound WS
- `ExternalMediaParams::new(app, host, format).encapsulation("rtp").transport("udp")` -- typed external media params

Builders validate required fields in `.build()` and return `Result`.

### Transport Modes (ARI)

ARI supports two transport modes selected via `AriConfigBuilder::transport()`:
- `TransportMode::Http` (default) -- separate HTTP for REST + WebSocket for events
- `TransportMode::WebSocket` -- unified WebSocket for both REST and events (requires Asterisk 20.14.0+)

### Resource Factory (ARI)

Solves the race condition between originate and event subscription:
```rust
let pending = client.channel(); // pre-generates channel ID, subscribes to events
let (handle, events) = pending.originate(params).await?; // StasisStart guaranteed buffered
```

Also available: `client.bridge()`, `client.playback()`.

### Call Tracker (AMI)

`CallTracker` correlates AMI events by UniqueID into `CompletedCall` records:
```rust
let (tracker, mut rx) = client.call_tracker();
let call = rx.recv().await; // CompletedCall with channel, unique_id, cause, events
```

### PBX Abstraction (umbrella)

High-level call management wrapping AmiClient + CallTracker:
```rust
let pbx = Pbx::new(client);
let call = pbx.dial("SIP/100", "SIP/200", None).await?;
call.wait_for_answer(Duration::from_secs(30)).await?;
call.hangup().await?;
```

### Event System

Protocol events implement `asterisk_rs_core::event::Event` (requires `Clone + Send + Sync + Debug + 'static`). Published via `EventBus<E>` (tokio broadcast). Consumed via `EventSubscription<E>::recv()` which handles lag by logging and skipping. Filtered subscriptions via `FilteredSubscription<E>` with predicate closures.

- AMI: `AmiEvent` enum with typed variants + `Unknown { event_name, headers }`. Serializable via serde. Event-generating actions collected via `EventListResponse` and `send_collecting()`.
- ARI: `AriEvent` enum with typed variants, serde `#[serde(tag = "type")]` + `#[serde(other)] Unknown`. Wrapped in `AriMessage` with application/timestamp/asterisk_id metadata. Serializable via serde.
- AGI: No event bus (synchronous request/response protocol)

### Reconnection

AMI and ARI use `ReconnectPolicy` from core:
- `ReconnectPolicy::exponential(initial, max)` -- default 1s initial, 60s max, jitter enabled
- `ReconnectPolicy::fixed(interval)` -- constant delay
- `ReconnectPolicy::none()` -- no retry

Background tasks manage reconnection state machine: `Disconnected -> Connecting -> Connected -> Reconnecting`.

AMI supports optional keep-alive pings via `AmiClientBuilder::ping_interval(Duration)`. When configured, the connection task sends periodic `PingAction` to detect dead TCP connections early. Disabled by default.

### Handle Pattern (ARI)

ARI resources use handles that bundle resource ID + client reference:

```rust
let handle = ChannelHandle::new(channel.id, client.clone());
handle.answer().await?;
handle.play("sound:hello").await?;
handle.hangup(None).await?;
```

Handles are `Clone + Debug`. Operations construct REST paths from the embedded ID. Query parameters use `url_encode` for safe encoding.

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

### Channel Variables

AMI events carry channel variables as `ChanVariable(name): value` headers on the wire. The codec extracts these into `RawAmiMessage.channel_variables: HashMap<String, String>` (separate from regular headers). Accessible via `get_variable(name)` on both `RawAmiMessage` and `AmiResponse`. Not propagated to typed `AmiEvent` variants.

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
| `crates/asterisk-rs-ari/src/client.rs` | AriClient transport-abstracted REST API |
| `crates/asterisk-rs-ari/src/transport.rs` | Transport abstraction: HttpTransport, WsTransport dispatch |
| `crates/asterisk-rs-ari/src/ws_transport.rs` | Unified WebSocket transport with REST request correlation |
| `crates/asterisk-rs-ari/src/event.rs` | AriEvent serde-tagged enum + supporting types |
| `crates/asterisk-rs-ari/src/pending.rs` | PendingChannel/Bridge/Playback race-free resource factory |
| `crates/asterisk-rs-ari/src/server.rs` | AriServer outbound WebSocket (Asterisk connects to app) |
| `crates/asterisk-rs-ari/src/media.rs` | MediaChannel for chan_websocket audio exchange |
| `crates/asterisk-rs-ari/src/resources/channel.rs` | ChannelHandle, OriginateParams, ExternalMediaParams |
| `crates/asterisk-rs-ami/src/tracker.rs` | CallTracker: AMI event correlation by UniqueID |
| `crates/asterisk-rs/src/pbx.rs` | Pbx high-level call abstraction (dial, hangup, wait_for_answer) |
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
- **Releases**: release-plz with consolidated releases (`release-plz.toml`)
- **Docs**: mdBook (user guide at `docs/`) + rustdoc (API reference)

## CI Matrix

| Job | Runs On | Toolchain | What |
|-----|---------|-----------|------|
| check | ubuntu | stable | `cargo check --workspace --all-targets --all-features` |
| fmt | ubuntu | nightly | `cargo fmt --all -- --check` |
| clippy | ubuntu | stable | `cargo clippy` with `-D warnings` |
| test | ubuntu/macos/windows | stable + 1.83 | `cargo test --workspace --all-features` (excludes test crate) |
| test-minimal | ubuntu | stable | `cargo test --workspace --no-default-features` (excludes test crate) |
| mock-tests | ubuntu | stable | `cargo test -p asterisk-rs-tests` (unit + mock) |
| integration | ubuntu | stable | live tests against Asterisk Docker with `--test-threads=1` |
| security | ubuntu | stable | Weekly + on Cargo.toml changes; cargo-deny + rustsec audit |
| coverage | ubuntu | stable | cargo-llvm-cov, uploads to codecov |
| semver | ubuntu | stable | cargo-semver-checks on PRs |
| docs | ubuntu | stable | rustdoc + mdbook, deploys to GitHub Pages |

## Testing

### Architecture

All tests live in the external `tests/` crate (no `#[cfg(test)]` in production code). Three test binaries:

| Binary | Tests | What |
|--------|-------|------|
| `unit` | ~880 | Pure data: codec, serialization, types, events, actions, responses, media protocol |
| `mock_integration` | ~210 | Mock servers: connection lifecycle, protocol exchanges, resource factory, media channel |
| `live_integration` | ~73 | Real Asterisk (Docker): full protocol coverage across AMI, AGI, ARI + tracker, transport |

### Test Infrastructure (`tests/src/mock/`)

- `MockAmiServer` — TCP listener with `accept_one` / `accept_loop`, banner, login helpers
- `MockAriServer` — HTTP + WebSocket on single port, route registration, event push
- `MockAgiClient` — connects to AGI server, sends environment, reads commands

### Running Tests

```sh
# unit + mock (fast, no network)
cargo test -p asterisk-rs-tests --test unit --test mock_integration

# live integration (requires Asterisk Docker)
cd tests && docker compose up -d
cargo test-live

# specific test
cargo test -p asterisk-rs-tests --test mock_integration -- mock_tests::ami::connect_and_login
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
