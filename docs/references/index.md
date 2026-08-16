# Reference index

External protocol and tooling facts are linked from the relevant design or user document rather
than copied wholesale. Record a vendored reference here only when reliable offline work requires
its exact content, along with source, version/date, owner, and refresh trigger.

Current authoritative external contracts are Asterisk AMI/AGI/ARI documentation, the Rust/Cargo
books, rust-clippy documentation, GitHub Actions security guidance, and release-plz documentation.

## Pinned contracts

- [`asterisk-22.9.0.md`](asterisk-22.9.0.md) pins the supported ARI and chan_websocket sources,
  digests, full generated upstream route/model inventory, exact local surface inventory, coverage
  boundary, owner, and refresh trigger.
- [`compatibility-0.8.md`](compatibility-0.8.md) records the intentional 0.7-to-0.8 break and the
  downstream and semver checks that protect later 0.8 releases.
