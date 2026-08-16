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
Reconnect behavior is development-proved with mocks; Asterisk-backed behavior is proved by
`just live`.

## Evidence

- `just test` exercises pure and mock failure paths.
- `just live` exercises the real PBX boundary serially.
- `just ci` checks all features, minimal features, rustdoc, generated docs, policy, and harness
  structure.

This library emits structured `tracing` events but owns no durable telemetry backend. Applications
choose subscribers, retention, alerts, and service-level objectives.
