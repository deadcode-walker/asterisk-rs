# Quality score

Updated 2026-08-16 from repository inspection and executable local gates. Grades describe current
evidence, not test volume.

| Domain | Grade | Current evidence | Gap / next action |
|---|---|---|---|
| Core types and event bus | A | unit tests cover conversion, backoff bounds, lag recovery, and secret redaction | keep MSRV proof on dependency changes |
| AMI | A- | codec/action/event tests plus mock connection, correlation, timeout, and reconnect paths | live carrier/PBX variants remain external evidence |
| AGI | A- | command injection rejection, response parsing, concurrency bounds, and mock sessions | add live cases when Asterisk changes AGI semantics |
| ARI | B+ | REST/WS mocks, typed events/resources, pending-race tests, media protocol tests | large event/resource files increase review cost; track only behavior-driven splits |
| PBX abstraction | B | typed dial options and AMI-backed implementation | lifecycle behavior has less boundary evidence than protocol crates |
| Documentation | B | mdBook and rustdoc build; references are generated | generator freshness must remain a required gate |
| Supply chain | B+ | Cargo.lock, cargo-deny, advisory workflow, rustls, no unsafe | immutable Actions pins and current lockfile required |
| Release operations | B | release-plz configuration and consolidated release ownership | publication remains externally credentialed and cannot be fully proved locally |

The completed modernization plan owns its historical gaps. Resolved items are removed rather than accumulated
as a permanent wish list.
