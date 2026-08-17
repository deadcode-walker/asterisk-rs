# Reliability

## Failure domains

- AMI: TCP connect/login, framing, action timeout/correlation, event lag, keep-alive, reconnect, and
  shutdown.
- AGI: listener bind, concurrency admission, malformed environment, command failure, channel hangup,
  handler failure, and shutdown with active sessions.
- ARI: REST status/JSON errors, event WebSocket disconnect/reconnect, unified request correlation,
  broadcast lag, resource-creation races, media framing, and outbound server shutdown.

## Bounded resources

- Event buses have finite broadcast capacity and report/skip lag rather than blocking producers.
- AGI concurrency is limited by a semaphore configured by the server builder.
- Client requests use explicit timeouts and pending request entries are removed on completion or
  disconnect.
- Reconnect delay is bounded by `ReconnectPolicy`; max retries can terminate recovery.

## Recovery and shutdown

Connection state is observable through watch channels. Background tasks receive explicit shutdown,
cancel pending work with typed errors, and must not keep user resources alive indefinitely.
Reconnect behavior is development-proved with mocks. `just live-full` additionally cuts a
caller-owned TCP relay in front of the real AMI endpoint and requires the client to recover against
the same Asterisk instance.

## Evidence

- `just test` exercises pure and mock failure paths.
- Live tests are marked ignored, so generic workspace and all-features test commands compile them
  without contacting a PBX.
- `just live-smoke` proves the owned-instance marker, AMI authentication/ping, ARI HTTP and unified
  WebSocket GET, isolated device/mailbox PUT round trips, and exact chan_websocket plaintext/JSON
  `MEDIA_START` schemas. `just live-full` runs the exhaustive ignored suite serially; `just live`
  remains its compatibility name.
- `just live-smoke-ci` and `just live-full-ci` own the repository Compose lifecycle. Attach-mode
  commands reuse a running repository fixture or require explicit mutation opt-in, expected branch,
  durable instance marker, run ID, AMI/ARI endpoints and credentials, and ARI application. The
  preflight reads the marker and Asterisk version before test mutation.
- Mutable smoke resources use the run ID in their Asterisk names. Exhaustive live tests remain
  serial, use the same typed configuration, and must clean up resources rather than turn missing
  fixture capabilities into warning-based passes.
- `just msrv` compiles every target and feature, including live-test code, on Rust 1.86.0, then runs
  the unit and mock suites without requiring Asterisk.
- `just ci` checks all features, minimal features, rustdoc, generated docs, policy, and harness
  structure.

This library emits structured `tracing` events but owns no durable telemetry backend. Applications
choose subscribers, retention, alerts, and service-level objectives.
