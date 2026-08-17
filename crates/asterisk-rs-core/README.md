# asterisk-rs-core

[![crates.io](https://img.shields.io/crates/v/asterisk-rs-core)](https://crates.io/crates/asterisk-rs-core)
[![docs.rs](https://img.shields.io/docsrs/asterisk-rs-core)](https://docs.rs/asterisk-rs-core)

Shared foundation for the asterisk-rs ecosystem. The AMI, AGI, and ARI crates
all depend on this crate for common types; it contains no protocol-specific logic.

## What it provides

### Errors

- `Error` — top-level error enum covering all failure modes
- `ConnectionError` — TCP connect failures, TLS errors, socket closed
- `AuthError` — login rejected, missing credentials, challenge failures
- `TimeoutError` — read/write/login timeouts
- `ProtocolError` — malformed frames, unexpected packet structure

### Event bus

- `Event` — marker trait required by the bus: `Clone + Send + Sync + Debug + 'static`
- `EventBus<E>` — broadcast hub; protocol crates publish into it internally
- `EventSubscription<E>` — bounded broadcast receiver that reports lag explicitly
- `FilteredSubscription<E>` — subscription with a predicate applied before delivery

### Reconnection

- `ReconnectPolicy` — three modes:
  - `ReconnectPolicy::exponential(initial, max)` — doubles delay each attempt, jitter applied to prevent thundering herd (default)
  - `ReconnectPolicy::fixed(interval)` — constant retry delay
  - `ReconnectPolicy::none()` — fail on first disconnect
- `ConnectionState` — observable state machine: `Disconnected → Connecting → Connected → Reconnecting`

### Credentials

- `Credentials` — username/secret pair; `Debug` impl redacts the secret so it never appears in logs or panic output

### Domain constants

Strongly-typed enums parsed from Asterisk protocol strings:

| Type | Examples |
|---|---|
| `HangupCause` | `NormalClearing`, `UserBusy`, `NoAnswer`, `NormalCircuitCongestion`, `NoRouteDestination`, … |
| `ChannelState` | `Down`, `Reserved`, `OffHook`, `Dialing`, `Ring`, `Up`, … |
| `DeviceState` | `Unknown`, `NotInUse`, `InUse`, `Busy`, `Unavailable`, … |
| `DialStatus` | `Answer`, `Busy`, `NoAnswer`, `Cancel`, `Congestion`, … |
| `CdrDisposition` | `Answered`, `NoAnswer`, `Busy`, `Failed` |
| `PeerStatus` | `Registered`, `Unregistered`, `Reachable`, `Unreachable`, … |
| `QueueStrategy` | `RingAll`, `LeastRecent`, `FewestCalls`, `RoundRobin`, … |
| `ExtensionState` | `NotInUse`, `InUse`, `Busy`, `Unavailable`, … |
| `AgiStatus` | `Success`, `InvalidCommand`, `DeadChannel`, `EndUsage` |

## Usage

You do not need to add this crate as a direct dependency unless you are building
a custom protocol integration. Applications that use these types directly should add
`asterisk-rs-core`; the protocol crates do not implicitly re-export its complete type surface.

---

Part of [asterisk-rs](https://github.com/deadcode-walker/asterisk-rs). MSRV 1.86. MIT OR Apache-2.0.
