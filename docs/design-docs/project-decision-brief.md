# asterisk-rs harness decision brief

Status: approved by repository owner direction and repository evidence on 2026-08-16.

## Outcome

Make asterisk-rs straightforward for a capable contributor or coding agent to understand, change,
exercise, review, and release without relying on stale chat context or a monolithic instruction
file. Preserve a small, dependable public API for async Asterisk AMI, AGI, and ARI integrations.

## Users and useful work

- Rust application developers embed protocol clients and FastAGI servers in Tokio services.
- Maintainers evolve Asterisk protocol coverage, compatibility, security, and releases.
- Contributors need focused local proof without a running PBX, plus an explicit Docker-backed live
  boundary for integration proof.

The representative useful paths are AMI connect/login/action/event correlation, AGI request/command
exchange, and ARI REST/WebSocket resource lifecycle.

## Current facts

- The repository is a Rust-only Cargo workspace with six packages, about 42,000 lines of Rust, and
  five publishable library crates.
- Cargo manifests and Cargo.lock are already required for crates.io publishing and release-plz.
- Unit and mock integration tests run without external services; live tests require Asterisk.
- Warm formatting, Clippy, unit, mock, workspace, feature-gate, dependency-policy, and typo checks
  complete locally in under a minute on the maintainer workstation.
- The old AGENTS.md was a 17 KB manual with stale MSRV facts; CLAUDE.md duplicated routing to it.
- GitHub Actions duplicated raw Cargo commands and used mutable action tags.

## Constraints

- Rust MSRV is an explicit public compatibility promise and must be tested, not inferred.
- The workspace remains Tokio-based and contains no unsafe Rust.
- Protocol crates may depend on core but not on one another; the umbrella crate owns composition.
- Secrets and credentials must remain redacted from Debug output and logs.
- Mock evidence is development proof; Asterisk-backed tests are the real protocol boundary.
- Publishing, GitHub releases, issue closure, and pull-request closure require observable evidence.

## Decisions

1. Keep Cargo as the sole dependency graph, build authority, and publishing authority.
2. Add a thin justfile as the sole human/agent command facade; CI calls the same recipes.
3. Do not add Bazel. This repository has no measured multi-language, remote-execution,
   cross-compilation, generated-runfile, or native build problem that would repay a second build
   graph. Reconsider only when one of those gaps is measured and an owner accepts BUILD metadata,
   toolchain, crate-universe, cache, and upgrade maintenance.
4. Replace the monolithic agent manual with a short AGENTS.md map, ARCHITECTURE.md, focused quality,
   reliability, security, product, and execution-plan documents.
5. Keep the harness in the public repository. These files are useful to all contributors and are
   project facts rather than Codex-specific prompt tricks.
6. Use stable Rust for primary development, exact MSRV for compatibility, and nightly only for
   rustfmt where a nightly-only formatting option is intentionally configured.
7. Prefer behavior and boundary checks over test-count or source-shape assertions.

## Alternatives rejected

- Keep raw Cargo commands only: lowest setup cost, but repeats policy across docs and CI and leaves
  no discoverable focused/full gate.
- Add Bazel beside Cargo: could provide hermetic toolchains and remote caching, but duplicates the
  dependency/build model while Cargo remains mandatory. Current feedback time and repository shape
  do not justify that lifecycle cost.
- Put harness material in a private Codex-only directory: hides architecture and evidence from human
  contributors and other tools, creating competing knowledge systems.

## Acceptance

- A newcomer can locate protocol ownership and dependency direction from AGENTS.md and
  ARCHITECTURE.md.
- `just --list` exposes focused, full, live, docs, dependency, and release-adjacent checks.
- `just check` is the bounded local/CI development gate and fails with actionable command output.
- `just ci` exercises the frozen candidate across formatting, lint, tests, feature gates, docs,
  dependency policy, security advisories, and repository structure.
- CI uses the command facade, least-privilege permissions, immutable action revisions, concurrency,
  and explicit required-job aggregation.
- Active complex work can be resumed from `docs/exec-plans/active/` without chat history.
