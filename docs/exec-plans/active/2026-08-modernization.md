# Repository harness and modernization

Governing execution contract: [`docs/PLANS.md`](../../PLANS.md). Before resuming this plan, reload
the active instruction chain and governing contract, verify the current tree and assumptions, then
continue from the recorded evidence.

## Purpose and non-goals

Replace stale repository guidance and unsafe automation, correct the protocol and lifecycle defects
found by the ground-up review, establish an intentional 0.8 compatibility boundary, and leave
`main` with reproducible local, cross-platform, live-Asterisk, release, and GitHub evidence.

This plan is the decision register for the full run. A reviewer finding may be implemented directly,
merged into a broader invariant, sequenced behind a prerequisite, or rejected with evidence. Nothing
is silently dropped because it was low severity or duplicated another report.

The non-goals are replacing Cargo with Bazel, adding infrastructure without a measured gap, claiming
mock proof as Asterisk proof, or treating local completion as authority to mutate GitHub or publish.
The current recoverable baseline for this correction is commit
`ea8875b298c89bd8b6fb7ae6f54f6572f41616c6`; later checkpoint identities are recorded below.
Material assumptions are that Cargo remains the published graph, Asterisk 22 remains the owned live
fixture, and the external GitHub state must be re-read before it is changed.

## Authority and side effects

The repository owner authorized local implementation, verification, documentation updates, and a
coherent local commit. Push, issue/PR closure, repository settings, merge, release, deployment, and
live-system mutation remain explicit later gates. For the 2026-08-17 harness correction, the owner
explicitly waived delegates and fresh reviewers; the main agent owns the source comparison,
mechanical gates, exact-diff inspection, and commit.

This interactive correction cycle has one implementation owner, no delegate concurrency, no blind
retry loop, and one complete-gate attempt after focused proof. A failed gate ends the tactic: record
the first causal failure, repair it if it remains within authority, and refreeze. The terminal reason
is either a green local harness commit or a concrete authority/external-state blocker.

## Progress

- [x] 2026-08-16: inventoried Git, packages, tests, workflows, issues, pull requests, dependencies,
  repository settings, installed tools, and baseline gates.
- [x] 2026-08-16: selected Cargo plus a thin just facade and rejected Bazel because no measured build
  scaling, polyglot, remote-execution, or hermeticity need justifies a second build graph.
- [x] 2026-08-16: installed the progressive-disclosure harness, replaced the old root instructions,
  removed CLAUDE.md, and committed the harness separately.
- [x] 2026-08-16: fixed issue 60, patched the vulnerable lock entry, bounded AMI frames, hardened
  authentication retry behavior, and committed the first protocol batch separately.
- [x] 2026-08-16: completed read-only reviews for Rust correctness, security, performance and bounds,
  concurrency and silent failure, simplification, tests, dependencies and MSRV, and CI/CD.
- [x] 2026-08-16: froze concurrent implementation after reviewers demonstrated that a moving tree
  caused inconsistent conclusions; collected exact handoffs for every partial change.
- [x] 2026-08-16: restored a deterministic green baseline and completed Slice 0 plus the
  foundational Slice 1/2 protocol, toolchain, dependency, and lifecycle work.
- [x] Replaced the workflow files after the local command contract was green; repository settings
  remain an external Slice 8 gate.
- [x] 2026-08-16: ran fresh exact-diff Rust, correctness, security, and performance reviews; resolved
  every in-scope finding and reran current, MSRV, semver, workflow, supply-chain, documentation,
  mock, unit, and live-Asterisk proof.
- [x] 2026-08-16: committed the bounded foundation checkpoint locally as `b0aae21`; push, monitor,
  and close external records only on later explicit instruction and only when pushed evidence
  supports closure.
- [x] 2026-08-16: migrate the repository harness to the refreshed Harness Engineering contract,
  preserve the incumbent toolchain, prove an intentional structural failure, run the frozen-candidate
  gate, and obtain fresh read-only review before recording the next exact checkpoint.
  The first frozen candidate was `git diff --binary` SHA-256 `d9dc2e57e1b4888b1bcff23a413949f1704c8596376aab4abbfad54fb47163eb`;
  `just ci`, `just semver`, 949 unit tests, and 252 mock tests passed. Two fresh reviewers found
  checker failure-path and parsing defects, which were corrected before the final gate and review.
  The enclosing local commit is the final exact checkpoint, avoiding a self-referential diff hash.
  Live Asterisk was not rerun because this slice changes only repository knowledge and enforcement.
  No GitHub state changed. When the owner resumes work, the selected next scope is the remaining
  Slice 1 compatibility and pinned-protocol contracts; it stays stopped until that instruction.
- [x] 2026-08-17: corrected the migration against every owner-supplied harness asset and reference,
  replaced the four semantic owners with repository-specific forms of the source contract, updated
  the active-plan schema and checker, ran the applicable complete gate, and inspected the exact diff
  without delegates or reviewers. `just ci` passed with 949 unit tests and 252 mock tests, plus
  formatting, strict Clippy, feature, supply-chain, documentation, harness, and workflow gates. Live
  Asterisk was not rerun because no Rust, protocol, fixture, or runtime behavior changed. The local
  commit enclosing this item is the recoverable checkpoint. The exact candidate was revalidated on
  resume with `just ci`: 949 unit tests and 252 mock tests passed together with formatting, strict
  Clippy, workspace/feature gates, supply-chain, generated documentation, harness, workflows, and
  typo checks.

## Surprises and discoveries

This was the pre-implementation review verdict and is retained as history, not as a description of
the current candidate. At that checkpoint no unsafe Rust, leaked repository credential, or active
known advisory remained in the graph, but the candidate was blocked by these findings:

- AMI command framing, deadline, keepalive, event-list, and stale-mutation correctness.
- hidden event loss that lets trackers, PBX calls, and pending resources claim complete state after
  missing lifecycle events.
- ARI REST method/wire-name errors and a chan_websocket API that does not match either the default
  plaintext protocol or the published JSON schema.
- unbounded or detached socket actors, queues, response bodies, handshakes, handlers, and trackers.
- missing behavioral proof for unified ARI WebSocket transport, PBX, pending-resource race handling,
  and several tests whose assertions currently accept every outcome.
- CI that can merge failed security, reports coverage without running the external behavior suite,
  and can publish an unverified manually selected branch with long-lived secrets.
- public API changes that require a deliberate 0.8 release rather than an accidental 0.7 patch.

The 2026-08-17 correction found a separate harness-process defect: the earlier migration followed
selective progressive disclosure even though the owner's acceptance boundary explicitly named every
asset and reference. The cache was current; the first causal error was incomplete source loading,
followed by a checker that still accepted the former ten-section plan contract. The correction uses
all named sources while retaining only verified repository facts and applicable optional machinery.

## Decision log

### Accepted foundations

- Cargo remains authoritative; just is the sole human/agent command map.
- Rust 1.97.1 is the exact development toolchain and 1.86.0 remains the exact MSRV.
- Edition 2024 and resolver 3 remain; compatible-resolution evidence proves resolver 3 is holding
  dependencies that require newer Rust.
- Keep curated high-signal Clippy denies. Do not enable blanket pedantic, nursery, or restriction
  groups, and delete clippy.toml if no live setting remains.
- Keep cargo-deny as the advisory/license/source/bans owner and cargo-shear for unused dependencies.
  Do not add cargo-audit, cargo-vet, Crev, nextest, cargo-hack, sccache, minimal-versions nightly, or
  custom library release profiles without a measured trigger.
- Retain Reqwest 0.13's explicit rustls/AWS-LC/platform-verifier selection. A reusable library must
  not install a process-global Rustls provider opportunistically. Prove the native build on Linux,
  macOS, Windows, and MSRV.
- Align secure WebSocket trust with HTTPS by using native roots or an explicit platform-verifier
  connector. Never disable certificate verification.
- Mark live PBX tests ignored by default, require the integration feature, and run them only through
  the serial, mutation-opted-in just recipe against an owned test instance.
- Treat the public dependency/API changes as a coordinated 0.8 release. Hide third-party error and
  transport types where practical so routine dependency upgrades stop becoming public semver breaks.

### Rejected or deferred tool choices

- Bazel is rejected for this six-crate library until measured scale or hermetic remote execution
  changes the economics.
- A required benchmark gate is deferred. Add focused resource/latency benchmarks first, observe
  stability, and gate only a demonstrated regression signal.
- One-review approval is rejected for branch protection while the repository is single-maintainer;
  it would deadlock maintenance. Conversation resolution and an exact aggregate CI check remain
  required.

- The refreshed harness skill does not justify a new scaffold: the required semantic owners already
  exist. Migrate them in place, remove the empty generic `docs/generated/` placeholder, and keep
  release-plz as the sole changelog owner.
- The owner-supplied assets define the semantic structure for the four existing owners; they are not
  copied as generic template prose. Repository evidence supplies every fact. Optional plugin,
  browser, telemetry, evaluator, and controller surfaces remain absent without a real trigger.
- This correction is performed by the main agent alone. The explicit no-review instruction overrides
  the normal fresh-review step for this slice only; later modernization slices retain it.

## Context and orientation

The workspace root owns Cargo, the exact toolchain selector, the just facade, workflows, and the root
instruction map. `ARCHITECTURE.md` routes shared, protocol, and composition changes into one of five
publishable crates. Public guidance and generated reference pages live under `docs/src/`; engineering
decisions and recovery state live under `docs/`. External tests are in the `tests` package, with mocks
as the default boundary and the Compose-managed Asterisk instance as live proof. Start each slice from
the useful-path table in `ARCHITECTURE.md`, then follow it to the smallest representative test.

## Milestones

### Slice 0: normalize and restore the proof baseline

- [x] Format the frozen partial tree and inspect every non-format change.
- [x] Repair line-aware AMI `Response: Follows` framing, including fragmented markers, blank output,
  marker substrings, colon-bearing output, and a coalesced next frame.
- [x] Reconcile the partial AMI actor rewrite rather than accepting its 857-line diff wholesale:
  preserve end-to-end deadlines and pre-wire cancellation, simplify actor-local state, return
  `OutcomeUnknown` only after wire execution may have begun, and keep deterministic encoder failures
  definitely unsent on the healthy connection.
- [x] Rerun exact current/MSRV check, format, strict Clippy, unit, mock, all-features, feature-matrix,
  deny, shear, docs, and harness gates before any architecture expansion.

### Slice 1: define the 0.8 compatibility and protocol contracts

- [ ] Record the 0.8 boundary for redirect, dependency-backed errors, media schemas, loss-aware events,
  model completeness, and private struct fields/builders; add downstream compile fixtures and make
  cargo-semver-checks blocking.
- [ ] Pin the supported Asterisk REST and chan_websocket schemas. Generate or mechanically compare
  model/route coverage instead of claiming the full Asterisk surface from handwritten partial types.
- [x] Change device-state and mailbox updates to PUT; encode every resource path/query segment; retain
  `callerId` and `appArgs`; implement redirect's required endpoint contract.
- [x] Make ARI HTTP, unified WebSocket, and outbound-session success semantics consistently 2xx and
  preserve response/status/body-read sources.
- [ ] Replace malformed AMI event defaults (`""`/zero) with required-field parsing that distinguishes
  malformed, unknown, and valid events. Preserve unknown ARI event type and raw payload.
- [ ] Complete ARI models from pinned fixtures and make future-facing models non-exhaustive or
  builder-based so fields can evolve without repeated breaks.

### Slice 2: one bounded actor and lifecycle policy

- [ ] Apply one policy to AMI, ARI, media, AGI, trackers, and servers: absolute admission-to-response
  deadlines, cancellation state, bounded in-flight work, bounded writes, explicit readiness/terminal
  error, owned JoinSet tasks, cooperative shutdown, bounded drain, then forced cancellation.
- [ ] AMI: definitive auth failures terminate startup with their typed cause; transient failures obey
  reconnect policy; retry budgets reset only after a stability window.
- [x] AMI: exact Ping ActionID and explicit pong deadline, skipped missed ticks, bounded Ping writes,
  fair reader/command scheduling, closed/deadline pending cleanup, and a maximum in-flight limit.
- [x] AMI collecting actions: resolve initial Error immediately, model Complete and Cancelled terminal
  states, and enforce aggregate byte plus event-count bounds.
- [ ] ARI: initial WebSocket readiness is observable; reconnect/backoff drains or expires commands;
  session and unified correlations share one private engine where that removes proven divergence.
- [ ] AGI/ARI servers: finite connection defaults, zero-value validation, handshake/prelude timeouts,
  peer identity, explicit external-bind opt-in, admission/authentication hook, task panic observation,
  and bounded graceful shutdown.
- [ ] Media: separate priority control from bounded audio, make flow-control state non-lossy, keep the
  socket actor nonblocking, and make enqueue-versus-transmit semantics explicit.
- [ ] Validate `ReconnectPolicy`, cap jitter after calculation, avoid zero-delay hot loops, and use
  saturating arithmetic for extreme attempts/durations.

### Slice 3: loss-aware state and resource bounds

- [ ] Expose event lag as a typed receive outcome. Stateful consumers must invalidate/reconcile rather
  than silently continue; retain a separately named explicitly lossy convenience API if useful.
- [ ] Rework CallTracker to use canonical event ID accessors, periodic time-driven eviction, bounded
  active calls, bounded per-call history, explicit truncation/loss counters, and observable completed
  call delivery loss.
- [ ] Remove the always-on tracker from Pbx, make Call ownership honest instead of shared competing
  clones, fail immediate Originate rejection, and use checked/saturating timeout conversion.
- [ ] Make pending ARI channel/bridge/playback filters exhaustive using centralized channel/bridge ID
  extraction; use restart-safe IDs rather than process-local counters.
- [ ] Bound AGI prelude line/total/variable counts and command line/multiline/total response size and
  time; require a blank prelude terminator and reject malformed lines without trimming meaningful data.
- [ ] Bound WebSocket messages/frames by protocol, reject media above 65,500 bytes before cloning,
  stream HTTP bodies under an application limit, and cap AMI collection bytes.
- [ ] Add conservative admission limits for every externally reachable server. Make cleartext remote
  ARI require an explicit insecure opt-in and support private CAs; add verified rustls AMI transport or
  document an explicit versioned proxy boundary if native TLS cannot be delivered safely in this run.

### Slice 4: exact chan_websocket and REST behavior

- [ ] Support both default plaintext and JSON control formats, or make JSON an explicit construction
  contract and carry `data`/`transport_data=f(json)` through external-media creation.
- [ ] Match every official JSON event: channel IDs, DTMF, queue length/watermarks/full state, bulk and
  pause state, buffering/mark correlation, and queue-drained identity.
- [ ] Match every command: parameterless HANGUP, correlated MARK_MEDIA, STOP buffering correlation,
  typed SET_MEDIA_DIRECTION, and documented passthrough restrictions.
- [ ] Remove full payload logging; log allowlisted event types, IDs, and byte counts only.
- [ ] Redact AMI secrets and PIN-bearing fields from Debug/serialization, minimize credential copies,
  zeroize credential-bearing URLs where practical, and add captured-log non-disclosure tests.

### Slice 5: rebuild tests around observable behavior

- [ ] Upgrade the mock ARI server to parse complete HTTP requests, Content-Length, auth/content-type,
  exact JSON, typed WebSocket frames, connection counts, same-port restart, and joined shutdown.
- [ ] Add unified ARI transport tests for all verbs, out-of-order correlation, interleaved events,
  malformed/duplicate IDs, reconnect, expiry, cancellation, and shutdown.
- [ ] Add PBX behavioral tests for exact Originate wire data, correlation, answer/hangup/failure,
  completion, lag, and timeout semantics.
- [ ] Prove pending-resource subscribe-before-create by delivering matching and unrelated events before
  the creation response.
- [ ] Replace the six vacuous ARI disconnect/reconnect/binary/close tests with one exact outcome each;
  remove tests of std/url implementation details and duplicate URL-encoding groups.
- [ ] Replace fixed sleeps and free-port TOCTOU helpers with bound-address handoff, notifications,
  paused time, and bounded awaits. Cancelled/panicked helper tasks must fail tests.
- [ ] Table-drive repetitive action contracts and route representative AMI event fixtures through the
  actual codec instead of directly constructing raw messages.
- [ ] Split live smoke from live full. Require explicit endpoints, mutation opt-in, a test-instance
  marker, deterministic fixtures, isolated resource IDs/cleanup, and no warning-based skips.
- [ ] Live matrix must prove supported Asterisk branches, AMI reconnect after a real transport cut,
  AGI session bounds, ARI HTTP and unified WS, media plaintext/JSON schemas, device/mailbox PUT, and
  cross-protocol behavior.
- [ ] Measure tracker/codec/media admission and latency at representative load; keep benchmarks
  informative until stable enough for a regression threshold.

### Slice 6: dependency, API, and deletion-first cleanup

- [x] Remove workspace Tokio `full`; enable exact production features per crate and dev/example-only
  runtime features separately. Remove Reqwest's unused `json` feature.
- [x] Remove or implement core's no-op serde feature in 0.8. Remove cargo-shear suppression afterward.
- [ ] Remove duplicate secret/config retention, inert top-level error aliases, dead ActionFailed
  semantics, redundant PBX tracker ownership, duplicate ARI models, and task-local Arc/Mutex state.
- [ ] Unify lifecycle-creating resource methods around handles and rename pending factories explicitly;
  require constructors for invalid-by-default parameter structs.
- [ ] Integrate core domain types through From/TryFrom/FromStr and typed accessors; replace
  ExtensionState's always-Some conversion.
- [ ] Delete the broken generated API-table scanner and link curated behavioral documentation to
  rustdoc, or replace it with a tested scope-aware generator. Do not retain duplicate/blank output.
- [ ] Split oversized AMI action/event and ARI test modules by protocol domain only after generator
  constraints are removed; preserve compatibility paths with re-exports.
- [ ] Add staged Rust-style aliases/deprecations for caller ID, Async AGI, FAX, and acronym-heavy names;
  remove old names only at the documented compatibility boundary.
- [ ] Establish and ratchet missing-doc coverage; convert drifting examples to compiled doctests or
  snippet tests and correct all README/package/version examples.
- [ ] Re-audit and rewrite the complete documentation surface against the proven 0.8 implementation:
  root and crate READMEs, rustdoc, mdBook concepts/how-to/reference material, examples, migration,
  security, reliability, support, and release guidance. Delete duplicated or aspirational claims,
  keep generated reference ownership explicit, test every command/snippet that can be automated, and
  have a fresh documentation/API reviewer verify information architecture and technical accuracy.

### Slice 7: replace CI, documentation deployment, and release automation

- [x] Replace five workflows with CI, docs, and release; optionally keep one scheduled/manual advisory
  discovery workflow. CI is the required owner of supply-chain policy.
- [x] CI triggers on pull_request, main push, and merge_group; uses exact Rust versions, `--locked`,
  least privilege, bounded timeouts, concurrency, Linux quality/live/coverage/semver, macOS and Windows
  behavior/platform proof, MSRV, and one aggregate `CI` job requiring every dependency to be success.
- [x] Coverage must execute the external unit/mock suite and publish a GitHub artifact. Remove Codecov
  until trend hosting is deliberately configured and upload failure is meaningful.
- [x] Prefer native rustup/Cargo/just. Delete marketplace wrappers for toolchain, typos, audit, deny,
  path filtering, semver, and Codecov. Pin every retained Action to a reviewed full SHA.
- [x] Add actionlint and zizmor to the command facade and required CI. End candidate CI with generated
  docs/harness checks and a clean-worktree drift assertion.
- [x] Pin the Asterisk image digest, bind ports to loopback, make config read-only/copied, use health
  checks, always collect failure logs, and always tear down volumes/orphans.
- [x] Docs run only after successful exact-SHA main CI; build under contents:read and grant Pages/OIDC
  only to deploy.
- [x] Release code runs only after successful exact-SHA main CI, checks out that immutable candidate,
  targets the release environment, uses short-lived GitHub App tokens and crates.io trusted
  publishing, and
  separate publish/release-PR jobs so release-PR concurrency cannot skip a publish candidate. Retain
  release-plz with corrected commit filters and independent package versions.
- [ ] Configure and verify the external protected release environment and `v*` tag ruleset before
  enabling release automation.

### Slice 8: repository settings and external closure

- [ ] Default GITHUB_TOKEN to read-only, disable Actions PR approval, enforce full-SHA/action allowlist,
  enable Dependabot security updates and private vulnerability reporting, and fix the disabled
  Discussions link.
- [ ] Replace the 14 fragile required contexts with only the GitHub-Actions-scoped `CI` aggregate after
  the new workflow is green; keep strict/up-to-date or merge-queue behavior and conversation resolution.
- [ ] Add the protected release environment and v* tag ruleset with only the release App plus emergency
  admin bypass.
- [ ] Configure Dependabot: Cargo patch/minor groups, majors separate, weekly Actions, Docker updates,
  and no unproved auto-merge.
- [ ] Run fresh Rust, security, performance, simplification, test, dependency, and CI exact-diff review.
  Resolve every valid critical through informational finding or record a concrete evidence-based reject.
- [ ] Commit only coherent green batches; push origin; monitor every GitHub check; fix forward and repeat
  review until the exact pushed SHA is green.
- [ ] Close issue 60 only with its pushed wire-format proof; close issue 57 only with patched lock and
  security evidence; close PRs 55 and 59 only after commenting that the pushed modernization supersedes
  them. Close any additional issue/PR only with equivalent evidence.

## Concrete steps

For each remaining slice, inspect `git status`, record its outcome and non-goals here, trace one
complete path from `ARCHITECTURE.md`, and iterate with `just test <filter>` followed by `just check`.
When the candidate is frozen, record `git rev-parse HEAD` plus the working-tree diff identity, run
`just ci`, run `just live` when the repository owns the required service, inspect the exact diff, and
request fresh read-only review unless the owner explicitly overrides it. Address valid findings and
rerun affected proof. Commit a coherent green slice after its required evidence is clean. Re-read
external GitHub state immediately before any later authorized push, settings change, closure, merge,
or release action.

## Validation and acceptance

No slice is complete because code exists. A slice is complete only when:

1. its focused behavioral tests pass;
2. formatting and strict Clippy pass on the changed targets;
3. exact Rust 1.86 compilation still passes;
4. the integrated deterministic suite remains green;
5. the exact diff receives a fresh independent review and all valid findings are resolved, unless
   the owner explicitly waives review for that slice;
6. public/API changes have semver and downstream-fixture evidence;
7. protocol changes have pinned-fixture proof and live proof where the repository owns the service.

Final acceptance additionally requires clean committed status, all local just gates, Linux/macOS/Windows
GitHub evidence, live Asterisk smoke and full evidence, green aggregate CI on the pushed SHA, protected
release/docs settings, and every closed external record linked to the evidence that resolved it.

## Idempotence and recovery

Commands and generated references are repeatable. Live Compose ownership includes cleanup on success,
failure, and cancellation. Commits remain small enough to diagnose and fix forward. Repository settings
change only after the replacement workflow is green so branch protection never points at a nonexistent
check. No issue, PR, package, tag, or release is deleted merely to make the dashboard appear clean.

## Interfaces and dependencies

This harness correction changes repository guidance and `scripts/check_harness.py`; it does not
change Rust source, public APIs, Cargo manifests, Cargo.lock, runtime behavior, or the supported
platform matrix. `AGENTS.md` routes execution, `ARCHITECTURE.md` owns useful paths and dependency
authority, `docs/README.md` indexes canonical knowledge, `docs/PLANS.md` owns recovery semantics,
the active plan owns current state, and `just harness` is the mechanical boundary. Cargo remains the
build/dependency owner and just remains its thin command facade.

## Outcomes and retrospective

The bounded checkpoint contains Slice 0 and the foundational Slice 1/2 work only. Final local proof
passed with 949 unit tests, 252 mock tests, and 73 serial live-Asterisk tests, plus exact Rust 1.86,
strict Clippy, generated documentation, cargo-deny, cargo-shear, actionlint, zizmor, and coordinated
0.8 semver checks. Fresh reviewers approved the remediated exact tree with no remaining in-scope
findings. Later slices and external repository settings remain open above.

Accepted bounded tradeoff: AMI outbound frames are prevalidated before actor admission and then
validated and encoded again at the socket boundary. The duplicate work is bounded by the 64 KiB
frame limit. Keep it for this candidate; an encode-once validated-frame design is deferred to later
simplification because changing the command API and actor ownership now would expand regression risk.

The 2026-08-17 harness correction read and applied the complete owner-supplied source set, rewrote
the four semantic owners around repository facts, upgraded the active-plan recovery contract, and
made their exact H2 structures mechanical. The main agent inspected the exact diff and `just ci`
passed; the owner explicitly waived delegates and reviewers for this correction. This documentation
and checker change did not require new live-Asterisk evidence and made no external change.

Complete the retrospective only after the pushed candidate and external records are settled. Record
reviewer yields, late defects, duplicated work, and harness changes here, then move this file to
`completed/`.
