# Product specification index

The mdBook pages under `docs/src/` are the canonical product specifications.

| Surface | Canonical pages | Acceptance boundary |
|---|---|---|
| Shared domain types | `docs/src/types.md` | generated type reference and unit conversion tests |
| AMI | `docs/src/ami/` | codec/action/event unit tests, mock lifecycle, live AMI tests |
| AGI | `docs/src/agi/` | request/response unit tests, mock sessions, live AGI tests |
| ARI | `docs/src/ari/` | serde/resource unit tests, HTTP/WS mocks, live ARI tests |
| Umbrella/PBX | root `README.md` and rustdoc | public API tests and semver checks |

Each behavior change updates its canonical guide and the smallest useful acceptance boundary.
