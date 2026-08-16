# asterisk-rs harness decision brief

Status: approved by repository owner direction and refreshed against the repository plus Harness
Engineering policy on 2026-08-16.

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

- The repository is a Rust-only Cargo workspace with six packages, about 47,500 lines of Rust, and
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

## Target and authority boundary

- Rust 1.86 is the public MSRV; Rust 1.97.1 is the exact development and primary CI toolchain.
- Linux, macOS, and Windows are the release-proven build targets. The owned live boundary is Asterisk
  22 on Linux through the repository Compose fixture. Android remains explicitly unsupported.
- Local repository edits, deterministic tests, documentation builds, and read-only review are within
  an authorized implementation slice. Pushes, repository settings, issue/PR closure, merge, package
  publication, GitHub release, and deployment are separate external effects.

## Acceptance map

| Outcome | Baseline | Cheapest representative proof | Complete proof | External authority |
|---|---|---|---|---|
| A fresh agent finds the correct owner | one root guide and five semantic owners exist | `just harness` plus a read-only fresh-session trace | fresh session identifies instruction scope, code path, command, and active plan without chat | Codex instruction discovery behavior |
| A contributor runs the right command | Cargo commands were previously duplicated across docs and CI | `just --list` and one filtered `just test` | `just ci` on the frozen tree and the same recipes in CI | supported host and CI runner availability |
| Protocol behavior is observable | unit and mocks need no service; live tests require Asterisk | focused unit or mock boundary test | `just live-ci` against the digest-pinned Asterisk fixture | Docker and the selected Asterisk image |
| A long task recovers from durable state | one active modernization plan exists | fresh reviewer states objective, exact checkpoint, risk, and next slice from repository files | a later session resumes without relying on this conversation | none |
| Mechanical boundaries remediate drift | harness checker validates files, instruction size, links, plans, and crate direction | intentional missing-owner failure produces an actionable error | `just ci` rejects generated, structural, dependency, and workflow drift | none |
| Delivery authority remains explicit | local work is four commits ahead of origin | agent distinguishes local proof from push/close/release authority | pushed exact SHA, protected settings, and external records are verified before mutation | GitHub repository owner and release environment |

## Toolchain ownership record

| Concern and single authority | Chosen path | No-extra or rejected alternative | Recovery and recheck trigger |
|---|---|---|---|
| Rust compatibility: workspace `rust-version`; exact selection: `rust-toolchain.toml` | Cargo/rustup with 1.97.1 development and 1.86 MSRV gates | no mise, Nix, Dev Container, or nightly formatter | retain prior selector/lock; recheck on compiler, MSRV, or target change |
| Dependency and build graph: `Cargo.toml` plus `Cargo.lock` | native Cargo workspace and resolver 3 | no second resolver, Bazel graph, or remote cache | Git restores the prior manifests/lock; reconsider only after a measured graph, hermeticity, or remote-execution failure |
| Public command surface: `justfile` | one thin, listable facade used by contributors, agents, and CI | raw Cargo-only routes were too easy to duplicate; no second task runner | every recipe exposes its Cargo/tool owner; remove just only after all callers move together |
| Live service lifecycle: `tests/docker-compose.yml` | digest-pinned isolated Asterisk fixture with bounded readiness and teardown | no service for unit/mock work; no shared PBX default | retain prior image digest; recheck on Asterisk branch/image or fixture-contract change |
| CI and documentation delivery: `.github/workflows/` | GitHub Actions calls repository recipes with least privilege and immutable action revisions | no second CI provider or agent-only gate | Git restores the prior workflow; recheck action/runner revisions, permissions, and protected settings |
| Release identity and changelogs: `release-plz.toml` | release-plz updates per-crate changelogs and umbrella release notes from reviewed commits | no independent version bumper or routine hand-edited `Unreleased` ledger | protected environment and trusted publishing remain external gates; recheck release-plz/crates.io contracts on update |
| Documentation: Markdown, rustdoc, mdBook, and `docs/generate.py` | repository-local public guide with mechanically checked generated references | no second portal or private Codex documentation tree | Git and generated-source inputs provide recovery; recheck when audience, publication, or generator needs change |
| Supply chain and verification: Cargo lints, cargo-deny, cargo-shear, Dependabot, and `scripts/check_harness.py` | one control per current advisory/license/unused-dependency/harness gap | no overlapping audit stack, test runner, or model grader without measured need | tool pins and policy are reviewed on update; remove a control when its signal no longer pays its lifecycle cost |

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
6. Use exact stable Rust 1.97.1 for primary development and exact Rust 1.86 for compatibility. Do not
   add nightly while stable rustfmt owns every configured option.
7. Prefer behavior and boundary checks over test-count or source-shape assertions.
8. Use reqwest 0.13's explicit [`rustls` backend](https://docs.rs/reqwest/0.13.4/reqwest/tls/).
   It selects the maintained AWS-LC provider and platform certificate verifier without requiring
   this library to install a process-global Rustls provider. Accept and document the provider's C/C++
   compiler requirement, supported target matrix, and Windows NASM/prebuilt-object behavior. CMake
   and Go are required only for FIPS builds in the selected AWS-LC release.
9. Treat the Rust 2024 migration, public Reqwest error-type upgrade, and removal of the inert core
   `serde` feature as a coordinated 0.8 compatibility boundary across the published workspace.
10. Keep workspace dependency versions centralized, but declare Tokio features at each consumer.
    Production crates enable only the runtime capabilities they use; examples and tests own their
    additional runtime, signal, and macro features.
11. Give every secure ARI WebSocket an explicit AWS-LC Rustls configuration and the same platform
    certificate verifier used by HTTPS. This avoids process-global provider selection, remains safe
    when downstream crates also enable Rustls's ring feature, and keeps HTTPS and WSS trust semantics
    aligned. Certificate verification is never disabled.
12. Treat Linux, macOS, and Windows as the release-proven TLS target set. Android is not currently a
    supported release target: rustls-platform-verifier requires its Kotlin/Gradle component plus
    application initialization before either HTTPS or WSS is constructed. Adding Android support
    requires documenting that host integration and exercising it in CI rather than hiding a runtime
    initialization requirement inside this reusable library.
13. Keep documentation public and repository-local. Public behavior is owned by code, rustdoc, and
    the mdBook; generated references have one checked generator. release-plz owns package changelogs
    and the umbrella release body, while ordinary changes own same-slice guides and migration notes.

## Alternatives rejected

- Keep raw Cargo commands only: lowest setup cost, but repeats policy across docs and CI and leaves
  no discoverable focused/full gate.
- Add Bazel beside Cargo: could provide hermetic toolchains and remote caching, but duplicates the
  dependency/build model while Cargo remains mandatory. Current feedback time and repository shape
  do not justify that lifecycle cost.
- Put harness material in a private Codex-only directory: hides architecture and evidence from human
  contributors and other tools, creating competing knowledge systems.
- Use reqwest's `rustls-no-provider` feature with ring: avoids AWS-LC's native build surface, but
  requires a provider to be installed before constructing a client. A reusable library should not claim the
  process-global provider merely to reduce its build graph.
- Keep Tokio `full` at workspace scope: convenient for examples, but it leaks filesystem, process,
  signal, multithreaded-runtime, and parking-lot features into every published protocol crate.
- Preserve core's no-op `serde` feature past 0.7: feature names are API promises, and an inert feature
  plus a permanent unused-dependency suppression is misleading rather than compatible behavior.
- Add a documentation portal or private agent manual: both would create another owner without an
  audience, search, versioning, or access problem that the repository-local stack cannot solve.

## 0.8 dependency compatibility boundary

All five published crates move to 0.8.0 together and require 0.8 protocol/core siblings. This is an
intentional incompatible baseline rather than a claim that the 0.7 API is preserved:

- `asterisk-rs-ari` replaces the public `AriError::Http(reqwest::Error)` payload with the crate-owned
  `HttpError` wrapper, removing Reqwest's concrete error type from the public API.
- Callers of `MediaChannel::from_accepted` must move their public tokio-tungstenite/tungstenite stream
  types from 0.29 to 0.30.
- `asterisk-rs-core` removes the advertised but inert `serde` feature and its unused optional
  dependency.
- `ChannelHandle::redirect` changes from
  `redirect(context: &str, extension: &str, priority: i64)` to `redirect(endpoint: &str)`, matching
  Asterisk's required endpoint-based redirect route. Callers must replace dialplan-location
  arguments with the destination endpoint string.
- FastAGI keeps its loopback bind default, but the server now defaults to at most 256 concurrent
  connections instead of an unbounded listener. A non-loopback `.bind(...)` must be followed by
  `.allow_external_bind(true)`; callers must also protect that listener at the network boundary
  because FastAGI has no native authentication.
- Published crates stop enabling Tokio's `full` feature by workspace-wide unification. Applications
  must enable any Tokio facilities they use directly instead of inheriting unrelated facilities from
  asterisk-rs.
- Secure ARI WebSockets move from the bundled WebPKI root set to the platform verifier used by HTTPS
  and select AWS-LC explicitly rather than relying on Rustls process-global provider inference. This
  can deliberately change which certificate chains are accepted.

Semver tooling should record these known 0.7-to-0.8 incompatibilities. It becomes a blocking
compatibility check for later 0.8 patch releases; it must not be muted merely to make this breaking
boundary appear compatible.

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
