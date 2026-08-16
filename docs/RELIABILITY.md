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
- Live tests are marked ignored, so generic workspace and all-features test commands compile them
  without contacting a PBX.
- `just live` opts into every ignored live test and exercises the real PBX boundary serially. When
  the repository's `tests/docker-compose.yml` Asterisk service is running, the recipe selects its
  loopback AMI/ARI ports and mutation opt-in. For any other isolated test PBX, callers must set
  `ASTERISK_TEST_ALLOW_MUTATION=1`, `ASTERISK_AMI_HOST`, `ASTERISK_AMI_PORT`, `ASTERISK_ARI_HOST`,
  and `ASTERISK_ARI_PORT` explicitly.
- `just msrv` compiles every target and feature, including live-test code, on Rust 1.86.0, then runs
  the unit and mock suites without requiring Asterisk.
- `just ci` checks all features, minimal features, rustdoc, generated docs, policy, and harness
  structure.

This library emits structured `tracing` events but owns no durable telemetry backend. Applications
choose subscribers, retention, alerts, and service-level objectives.
