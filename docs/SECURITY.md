# Security engineering

The public disclosure policy is the root [SECURITY.md](../SECURITY.md). This file records code and
automation trust boundaries.

## Trust boundaries

- AMI and AGI consume line-oriented data from a PBX or network peer.
- ARI consumes HTTP/WebSocket responses and constructs paths/queries from caller input.
- Builders consume credentials, network addresses, timeouts, and transport choices.
- GitHub Actions consume repository content, dependency metadata, tokens, and publishing secrets.

## Invariants

- Workspace Rust forbids unsafe code.
- Credentials and authentication actions never reveal secrets through Debug or tracing.
- Line protocols reject CR/LF injection in command names and arguments.
- ARI user-controlled path and query values are percent-encoded at the request boundary.
- Network operations use configured timeouts and bounded retry policy.
- Release credentials are available only to the release job with explicit minimal permissions.

## Dependency ownership

Cargo.toml and Cargo.lock are authoritative. `cargo deny check` enforces advisories, licenses,
sources, and duplicate-version policy. Dependabot proposes grouped Cargo and Actions updates; a
maintainer must validate behavior and close superseded proposals.

## Evidence

- `just lint` denies compiler and Clippy warnings for every target and feature.
- `just supply-chain` checks advisories, licenses, bans, and registry sources.
- `just test` covers credential redaction, injection rejection, malformed input, authorization
  failures, timeouts, and disconnect cleanup.
- GitHub workflows use least-privilege permissions and immutable action revisions.

No local gate proves the security of a deployed Asterisk configuration, network perimeter, stored
application logs, GitHub organization policy, or external publishing credentials.
