---
description: "Rust conventions for this workspace. Read when writing or modifying Rust code."
globs:
  - "*.rs"
  - "Cargo.toml"
---

# Rust Conventions

## Error handling

- Use `thiserror` for error types. Every crate has its own `Error` enum in `error.rs`.
- Propagate with `?`. Map context with `.map_err()` or `.context()` where the source alone is unclear.
- No `.unwrap()` — workspace lints deny it. Use `.expect("reason")` only for provably infallible cases (static regex, known-good parse).
- Return `Result<T, Error>` from all public APIs. Never panic in library code.

## Async

- Runtime is `tokio` with full features. All async code is tokio-native.
- Connection types own their `tokio::task` handles and clean up on drop.
- Use `tokio::select!` for concurrent operations, not `futures::join!` unless all branches must complete.
- Cancel safety: document whether async functions are cancel-safe in their doc comments.
- AMI connection task re-authenticates after every reconnect (Login is re-sent automatically).
- ARI HTTP client uses 10s connect timeout + 30s request timeout.

## Types

- Prefer newtypes over raw primitives for domain concepts (`ActionId(String)` not bare `String`).
- Derive `Debug, Clone` on all public types. Add `Serialize, Deserialize` where wire format applies.
- `#[non_exhaustive]` on all public enums that may grow.
- `Serialize` on event types to support logging and forwarding.
- `PartialEq` on AMI event and response types for assertion and matching.

## Crate boundaries

- `asterisk-rs-core` owns shared types. Other crates depend on core, never on each other.
- Each protocol crate (ami, agi, ari) is independently usable.
- `asterisk-rs` is the umbrella re-export. It adds no logic, only pub use.

## Testing

- No `#[cfg(test)]` or inline test modules in production crates. All tests live in the external `tests/` crate (`asterisk-rs-tests`).
- Unit, mock integration, and live integration tests are separate binaries in `tests/`.
- Run tests with `cargo test -p asterisk-rs-tests`, never with per-crate `cargo test -p asterisk-rs-ami`.

## Build

```bash
cargo check --workspace                # type check
cargo clippy --workspace -- -D warnings  # lint
cargo test --workspace                 # test
cargo doc --workspace --no-deps        # docs
```

## Patterns

- `FilteredSubscription` wraps `EventSubscription` with a predicate closure for selective event delivery.
- `EventListResponse` collects multi-event action results via `send_collecting()`.
- `AriMessage` wraps `AriEvent` with common metadata fields (application, timestamp, asterisk_id).
- `url_encode()` in ARI client for percent-encoding user-provided query parameter values.
- `ShutdownHandle` returned from AGI server builder for graceful shutdown.
- AMI `RawAmiMessage.output` captures multi-line command output from `Response: Follows`.
