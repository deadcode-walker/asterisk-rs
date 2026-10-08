# 0.8 compatibility boundary

Status: normative for the 0.8 line. Maintainers update this document when a public API or supported
protocol contract changes.

Version 0.8 deliberately breaks the 0.7 surface once so later 0.8 patch releases can be checked as
compatible. The coordinated boundary includes:

- endpoint-based `ChannelHandle::redirect`, replacing dialplan context/extension/priority arguments;
- crate-owned HTTP error context instead of a public Reqwest error payload;
- removal of core's inert `serde` feature;
- consumer-specific Tokio features instead of workspace-wide `full`; applications must enable
  the Tokio facilities they use directly;
- tokio-tungstenite/tungstenite 0.30 stream types for `MediaChannel::from_accepted`, replacing 0.29;
- platform certificate verification for secure ARI WebSockets, replacing bundled WebPKI roots and
  aligning WSS trust with HTTPS; this can change which certificate chains are accepted;
- the exact pinned chan_websocket JSON event and command schemas, including lossless unknown ARI
  events and explicitly classified malformed AMI events;
- complete fields for each implemented ARI response model against the pinned Asterisk 22.9.0
  fixtures, without claiming all 82 upstream models or all 102 routes;
- private request fields constructed through `OriginateParams`, `ExternalMediaParams`, and
  `ConfigTuple` constructors and fluent methods; and
- non-exhaustive response models and protocol enums so upstream additions do not force callers into
  exhaustive construction or matching.

`just downstream` compiles the intended construction and matching surface from the external tests
crate. `just semver` compares every published workspace crate with its published predecessor. The
GitHub `semver` job is a required dependency of the aggregate `CI` job, making later 0.8 compatibility
regressions blocking rather than advisory.
